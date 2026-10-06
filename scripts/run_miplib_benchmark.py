"""MIPLIB 2017 の小〜中規模の問題で、このクレートの分枝限定法と HiGHS を比べる。

問題は https://miplib.zib.de/WebData/instances/<name>.mps.gz から `<repo>/.miplib_cache/` に取得する
(一度だけ。キャッシュは .gitignore 済み)。各問題を両ソルバーでそれぞれ別のサブプロセスで解く:

- HiGHS: 既定の設定 + `time_limit`。`Highs.run()` の時間を測る。
- このクレート: MPS は HiGHS で読み、`benchmark_highs._build_our_model` でモデルを作る (整数性も渡す)。
  `model.solve(time_limit=...)` の時間だけを測る。親プロセスは時間上限 + 余裕で打ち切る。

結果は問題ごとに `--out` (JSON) に書き足すので、中断しても同じコマンドで再開できる。
`--table` は JSON から Markdown の表だけを作り直す。

使い方: python scripts/run_miplib_benchmark.py [--time-limit 60] [--only NAME ...] [--table]
"""

from __future__ import annotations

import argparse
import gzip
import json
import math
import os
import shutil
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
BASE_URL = "https://miplib.zib.de/WebData/instances/"

# MIPLIB 2017 の collection から選んだ、HiGHS が数秒〜数十秒で解く程度の問題 (正しさと速度の確認用)。
PROBLEMS = [
    "gen-ip002", "gen-ip054", "pk1", "gr4x6", "markshare_4_0", "mas74", "mas76", "neos5", "p0201",
    "misc07", "flugpl", "dcmulti", "gt2", "khb05250", "qnet1", "rout", "10teams", "air03", "blend2",
    "fiber", "noswot", "qiu", "neos-911970", "30n20b8", "binkar10_1", "neos-860300", "eil33-2", "nw04",
    "assign1-5-8", "beavma", "blp-ar98", "ran14x18-disj-8", "h80x6320d", "supportcase26", "timtab1",
    "bppc4-08", "cvs16r128-89", "mik-250-20-75-4", "neos-1456979", "graph20-20-1rand",
]


def fetch(name: str, cache: Path) -> Path:
    """`name` の MPS を取得して展開し、そのパスを返す。"""
    cache.mkdir(parents=True, exist_ok=True)
    mps = cache / f"{name}.mps"
    if mps.exists() and mps.stat().st_size > 0:
        return mps
    gz = cache / f"{name}.mps.gz"
    with urllib.request.urlopen(BASE_URL + f"{name}.mps.gz", timeout=120) as r, open(gz, "wb") as f:
        shutil.copyfileobj(r, f)
    with gzip.open(gz, "rb") as src, open(mps, "wb") as dst:
        shutil.copyfileobj(src, dst)
    gz.unlink()
    return mps


def worker_highs(mps: str, time_limit: float) -> None:
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", time_limit)
    h.setOptionValue("threads", 1)
    h.readModel(mps)
    t = time.perf_counter()
    h.run()
    t = time.perf_counter() - t
    info = h.getInfo()
    print(json.dumps({
        "status": h.modelStatusToString(h.getModelStatus()),
        "objective": info.objective_function_value,
        "bound": info.mip_dual_bound,
        "nodes": info.mip_node_count,
        "time": t,
    }))


def worker_ours(mps: str, time_limit: float) -> None:
    from enomoto_solver import benchmark_highs as bh

    _, lp = bh._load_lp(Path(mps))
    m, _, _ = bh._build_our_model(lp)
    t = time.perf_counter()
    r = m.solve(None, False, time_limit, None, None)
    t = time.perf_counter() - t
    print(json.dumps({
        "status": r["status"],
        "objective": r["objective"],
        "bound": r["best_bound"],
        "nodes": r["nodes"],
        "time": t,
    }))


def run_worker(kind: str, mps: Path, time_limit: float) -> dict:
    cmd = [sys.executable, __file__, f"--worker-{kind}", str(mps), "--time-limit", str(time_limit)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=time_limit * 1.5 + 60)
    except subprocess.TimeoutExpired:
        return {"status": "killed", "objective": None, "bound": None, "nodes": None, "time": None}
    for line in reversed(p.stdout.splitlines()):
        if line.startswith("{"):
            return json.loads(line)
    return {"status": f"crash rc={p.returncode}", "objective": None, "bound": None, "nodes": None, "time": None,
            "stderr": p.stderr[-2000:]}


def solved(r: dict) -> bool:
    return r.get("status") in ("Optimal", "optimal")


def make_table(results: dict, time_limit: float) -> str:
    names = [n for n in PROBLEMS if n in results] + [n for n in results if n not in PROBLEMS]
    rows = []
    sg = {"highs": [], "ours": []}
    n_solved = {"highs": 0, "ours": 0}
    wrong = []
    for n in names:
        h, o = results[n]["highs"], results[n]["ours"]
        for k, r in (("highs", h), ("ours", o)):
            t = r["time"] if solved(r) and r["time"] is not None else time_limit
            sg[k].append(t)
            n_solved[k] += solved(r)
        match = ""
        if solved(h) and solved(o):
            ok = abs(h["objective"] - o["objective"]) <= 1e-4 * max(1.0, abs(h["objective"])) + 1e-6
            match = "✓" if ok else "✗"
            if not ok:
                wrong.append(n)
        elif solved(h) != solved(o):
            match = "-"
        fmt = lambda r: f"{r['time']:.2f}" if solved(r) else ("t" if "time" in str(r["status"]).lower() or r["status"] == "killed" else str(r["status"]))
        fobj = lambda r: f"{r['objective']:.6g}" if r.get("objective") is not None else "-"
        rows.append(f"| {n} | {fmt(h)} | {fmt(o)} | {fobj(h)} | {fobj(o)} | {h.get('nodes')} | {o.get('nodes')} | {match} |")

    def shifted_geomean(ts, shift=10.0):
        return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift if ts else float("nan")

    out = [
        "# MIPLIB 2017 (小〜中規模): enomoto_solver vs HiGHS",
        "",
        f"- 実行日時: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}",
        f"- 時間上限 {time_limit:g} 秒、HiGHS は 1 スレッド、既定の相対ギャップ (両者 1e-4)",
        "- 時間は求解のみ (MPS 読み込み・モデル構築を除く)。`t` = 時間上限",
        "",
        "| 指標 | HiGHS | enomoto |",
        "|---|---:|---:|",
        f"| 解けた問題数 (/{len(names)}) | {n_solved['highs']} | {n_solved['ours']} |",
        f"| Shifted geomean [s] (shift 10、未解決は上限扱い) | {shifted_geomean(sg['highs']):.2f} | {shifted_geomean(sg['ours']):.2f} |",
        f"| 目的値の不一致 | | {len(wrong)} {wrong if wrong else ''} |",
        "",
        "| 問題 | HiGHS [s] | enomoto [s] | HiGHS 目的値 | enomoto 目的値 | HiGHS ノード | enomoto ノード | 一致 |",
        "|---|---:|---:|---:|---:|---:|---:|:---:|",
    ] + rows
    return "\n".join(out) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--time-limit", type=float, default=60.0)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--out", type=Path, default=REPO_ROOT / "benchmarks" / "miplib_results.json")
    ap.add_argument("--cache-dir", type=Path, default=REPO_ROOT / ".miplib_cache")
    ap.add_argument("--table", action="store_true")
    ap.add_argument("--rerun-ours", action="store_true", help="HiGHS の結果は再利用し、このクレートだけ解き直す")
    ap.add_argument("--worker-highs")
    ap.add_argument("--worker-ours")
    a = ap.parse_args()
    if a.worker_highs:
        return worker_highs(a.worker_highs, a.time_limit)
    if a.worker_ours:
        return worker_ours(a.worker_ours, a.time_limit)
    results = json.loads(a.out.read_text()) if a.out.exists() else {}
    if not a.table:
        names = a.only or PROBLEMS
        for n in names:
            if n in results and not a.rerun_ours:
                continue
            mps = fetch(n, a.cache_dir)
            h = results.get(n, {}).get("highs") if a.rerun_ours else None
            if h is None:
                h = run_worker("highs", mps, a.time_limit)
            o = run_worker("ours", mps, a.time_limit)
            results[n] = {"highs": h, "ours": o}
            print(f"{n:24s} highs={h['status']:>10s} {h['time'] if h['time'] is not None else '-':>8} obj={h['objective']}  "
                  f"ours={o['status']:>10s} {o['time'] if o['time'] is not None else '-':>8} obj={o['objective']}", flush=True)
            a.out.parent.mkdir(parents=True, exist_ok=True)
            a.out.write_text(json.dumps(results, indent=1))
    md = make_table(results, a.time_limit)
    a.out.with_suffix(".md").write_text(md)
    print(md)


if __name__ == "__main__":
    main()
