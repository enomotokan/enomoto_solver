"""マルチフロンタル法 (内点法の疎 Cholesky / LDLᵀ) の版ごとの比較 (内点法 + クロスオーバー、`root_solver="ipm_crossover"`)。

各問題で、版 (`--variants 名前=環境変数,...`) を 1 回ずつ交互に `--reps` 回実行し (計算機の状態の揺れが版の差に
偏らないように)、1 回ごとに 1 行の JSON を `--out` に追記する (中断しても同じコマンドで再開できる)。

1 回の実行は `scripts/crossover_bench/run.py` のワーカーを新しいプロセスで動かし (MPS の読み込みとモデル構築は計時
しない)、`ENOMOTO_DEBUG_IPM=1` の標準エラーから次を読む:
  - `time`: `PyModel.solve` の実時間 (前処理・内点法・クロスオーバー・後処理)
  - `ipm_factor`: 内点法の数値分解の累計 (`IPM profile ... factor=`、内点法を複数回呼べばその和)
  - `ipm_total`: 内点法の Newton 系の部分の累計 (`IPM profile total=`)
  - `backend`: 分解に使ったもの (`NormalKkt: backend=` / `AugKkt: backend=`)
  - `maxrss_mb`: ワーカーのプロセスの最大常駐メモリ
`--mf-prof` なら `ENOMOTO_DEBUG_MF=1 ENOMOTO_DEBUG_REFINE=1` も付けて、分解ごとの内訳
(前線の組み立てと extend-add・対角ブロックの分解・三角行列の解法・更新行列の計算、スレッドの和) を合計する
(計時の手間が入るので、時間の比較とは別に実行する)。

使い方 (リポジトリのルートで、.venv に enomoto_solver と highspy を入れた状態):
    python scripts/mf_bench/run.py --only qap15 scpm1 --variants base= packed=ENOMOTO_T_MF_PACKED=1 \
        --reps 3 --out benchmarks/mf_bench/ab.jsonl
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts" / "crossover_bench"))

WRAP = (
    "import resource, runpy, sys\n"
    "sys.argv = sys.argv[1:]\n"
    "try:\n"
    "    runpy.run_path(sys.argv[0], run_name='__main__')\n"
    "finally:\n"
    "    print('MAXRSS_KB', resource.getrusage(resource.RUSAGE_SELF).ru_maxrss, file=sys.stderr, flush=True)\n"
)


def parse(stdout: str, stderr: str) -> dict:
    res: dict = {}
    for line in stdout.splitlines():
        if line.startswith("RESULT "):
            r = json.loads(line[7:])
            res.update(time=r["time"], status=r["status"], obj=r["obj"])
    fac = tot = 0.0
    n_prof = n_fac = 0
    fac_t = 0.0
    prof = [0.0, 0.0, 0.0, 0.0]
    backends = []
    for line in stderr.splitlines():
        m = re.match(r"IPM profile total=([\d.]+)s factor=([\d.]+)s", line)
        if m:
            tot += float(m.group(1))
            fac += float(m.group(2))
            n_prof += 1
        m = re.match(r"(NormalKkt|AugKkt): backend=(\w+)", line)
        if m:
            backends.append(f"{m.group(1)}:{m.group(2)}")
        m = re.match(r"FACTOR mf ok=\w+ t=([\d.]+)s prof=\[([^\]]*)\]", line)
        if m:
            n_fac += 1
            fac_t += float(m.group(1))
            for i, v in enumerate(m.group(2).split(",")):
                prof[i] += float(v)
        if line.startswith("MAXRSS_KB "):
            res["maxrss_mb"] = int(line.split()[1]) / 1024
    if n_prof:
        res.update(ipm_factor=fac, ipm_total=tot, ipm_calls=n_prof)
    res["backend"] = sorted(set(backends))
    if n_fac:
        res.update(mf_factors=n_fac, mf_factor_time=fac_t, mf_prof=prof)
    return res


def run_one(mps: Path, timeout: float, mem_gb: float, env_extra: dict, mf_prof: bool) -> dict:
    cmd = [sys.executable, "-c", WRAP, str(REPO / "scripts/crossover_bench/run.py"), "--worker", str(mps), "ipm_crossover"]
    env = dict(os.environ)
    env["ENOMOTO_DEBUG_IPM"] = "1"
    if mf_prof:
        env["ENOMOTO_DEBUG_MF"] = "1"
        env["ENOMOTO_DEBUG_REFINE"] = "1"
    env.update(env_extra)
    lim = int(mem_gb * (1 << 30))
    pre = (lambda: __import__("resource").setrlimit(__import__("resource").RLIMIT_AS, (lim, lim)))
    t0 = time.perf_counter()
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env, preexec_fn=pre)
    except subprocess.TimeoutExpired:
        return {"status": "timeout", "time": timeout}
    res = parse(p.stdout, p.stderr)
    res["wall"] = time.perf_counter() - t0
    if "status" not in res:
        res["status"] = "crash"
        res["stderr"] = p.stderr[-2000:]
    return res


def main() -> None:
    from run import materialize  # scripts/crossover_bench/run.py

    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="mittelmann")
    ap.add_argument("--only", nargs="+", required=True)
    ap.add_argument("--variants", nargs="+", required=True, help="名前=環境変数=値,環境変数=値 (名前= だけなら既定)")
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--time-limit", type=float, default=600.0)
    ap.add_argument("--mem-gb", type=float, default=13.5)
    ap.add_argument("--mf-prof", action="store_true")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    variants = {}
    for v in args.variants:
        name, _, envs = v.partition("=")
        variants[name] = dict(e.split("=", 1) for e in envs.split(",") if e)
    done: dict = {}
    if args.out.exists():
        for line in args.out.read_text().splitlines():
            r = json.loads(line)
            done[(r["problem"], r["variant"])] = done.get((r["problem"], r["variant"]), 0) + 1
    args.out.parent.mkdir(parents=True, exist_ok=True)
    for name in args.only:
        if all(done.get((name, v), 0) >= args.reps for v in variants):
            continue
        work = Path(tempfile.mkdtemp(prefix="mfb_"))
        try:
            mps = materialize(args.set, name, work)
            for rep in range(args.reps):
                for v, env in variants.items():
                    if done.get((name, v), 0) > rep:
                        continue
                    r = run_one(mps, args.time_limit, args.mem_gb, env, args.mf_prof)
                    rec = {"problem": name, "variant": v, "rep": rep, "env": env, "prof_mode": args.mf_prof, **r}
                    with open(args.out, "a") as f:
                        f.write(json.dumps(rec) + "\n")
                    print(f"{name:20s} {v:12s} rep={rep} {r['status']:10s} time={r.get('time', 0):8.2f}s "
                          f"factor={r.get('ipm_factor', float('nan')):8.2f}s rss={r.get('maxrss_mb', 0):7.0f}MB {r.get('backend')}", flush=True)
                    if r["status"] in ("timeout", "crash"):
                        break
        finally:
            shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
