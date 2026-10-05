"""内点法 + クロスオーバー (root_solver="ipm_crossover") と傾き・切片双対二段解法 (既定の "simplex") の
求解時間の比較。

各問題・各解法を新しいプロセスで解き (MPS の読み込みとモデル構築は計時しない。`Model.solve` 相当の
`PyModel.solve` の前後の実時間で、前処理・後処理を含む)、1 行ずつ JSON Lines で `--out` に追記する
(中断しても同じコマンドで再開できる)。正否は benchmarks/paper/per_problem.csv の基準値
(reference_obj) と相対 1e-6 で比べる。

使い方 (リポジトリのルートで、.venv に enomoto_solver と highspy を入れた状態):
    python scripts/crossover_bench/run.py --set netlib --out benchmarks/crossover/results.jsonl
    python scripts/crossover_bench/run.py --set kennington mittelmann --min-presolved-rows 5000 ...
    python scripts/crossover_bench/run.py --sizes-only --set kennington mittelmann   # 前処理後の大きさだけ
"""
from __future__ import annotations

import argparse
import bz2
import csv
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

CACHES = {
    "netlib": REPO / ".netlib_cache",
    "kennington": REPO / ".kennington_cache",
    "mittelmann": REPO / ".mittelmann_cache",
}
# この計算機のメモリ (15 GB) に載らない Mittelmann の 4 問 (論文用ベンチマークと同じ除外)。
MITTELMANN_EXCLUDE = {"thk_48", "L2CTA3D", "dlr2", "Dual2_5000"}
METHODS = {"slope_intercept": "simplex", "ipm_crossover": "ipm_crossover", "race": "auto"}


def problems(set_name: str) -> list[str]:
    if set_name == "mittelmann":
        from run_mittelmann_benchmark import PROBLEMS
        return [p[0] for p in PROBLEMS if p[0] not in MITTELMANN_EXCLUDE]
    return [l.strip() for l in (CACHES[set_name] / "problems.txt").read_text().splitlines() if l.strip()]


def materialize(set_name: str, name: str, workdir: Path) -> Path:
    if set_name != "mittelmann":
        return CACHES[set_name] / "mps" / f"{name}.mps"
    from run_mittelmann_benchmark import PROBLEMS, materialize_mps
    rel = next(p[1] for p in PROBLEMS if p[0] == name)
    raw = CACHES["mittelmann"] / "raw" / rel.replace("/", "__")
    return materialize_mps(CACHES["mittelmann"], rel, raw, workdir)


def worker(mps: str, solver: str, sizes_only: bool) -> None:
    import highspy
    from enomoto_solver import _core
    from enomoto_solver.benchmark_highs import _build_our_model

    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(mps)
    lp = h.getLp()
    model, _, _ = _build_our_model(lp)
    del h, lp
    if sizes_only:
        os.environ["ENOMOTO_DEBUG_PRESOLVE_SIZE"] = "1"
        os.environ["ENOMOTO_PRESOLVE_ONLY"] = "1"
    t0 = time.perf_counter()
    out = model.solve(root_solver=solver, distinguish_infeasible_unbounded=True)
    t = time.perf_counter() - t0
    print("RESULT " + json.dumps({"time": t, "status": out["status"], "obj": out["objective"],
                                  "events": [[n, s] for n, s in _core.last_solve_events()]}), flush=True)


def reference_objs() -> dict:
    ref = {}
    with open(REPO / "benchmarks/paper/per_problem.csv") as f:
        for r in csv.DictReader(f):
            if r["reference_obj"]:
                ref[(r["set"], r["problem"])] = float(r["reference_obj"])
    return ref


def run_one(mps: Path, solver: str, timeout: float, mem_gb: float, sizes_only: bool = False) -> dict:
    cmd = [sys.executable, __file__, "--worker", str(mps), solver] + (["--sizes-only"] if sizes_only else [])
    env = dict(os.environ)
    lim = int(mem_gb * (1 << 30))
    pre = (lambda: __import__("resource").setrlimit(__import__("resource").RLIMIT_AS, (lim, lim)))
    t0 = time.perf_counter()
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env, preexec_fn=pre)
    except subprocess.TimeoutExpired:
        return {"status": "timeout", "time": timeout}
    wall = time.perf_counter() - t0
    res = {"wall": wall}
    for line in p.stdout.splitlines():
        if line.startswith("RESULT "):
            res.update(json.loads(line[7:]))
    for line in p.stderr.splitlines():
        if line.startswith("PRESOLVE_SIZE"):
            res["presolve_size"] = {k: int(v) for k, v in (kv.split("=") for kv in line.split()[1:])}
    if "status" not in res:
        # 異常終了 (メモリ上限の超過など): 解けなかったものとして制限時間で記録する。
        res["status"] = "crash"
        res["time"] = timeout
        res["stderr"] = p.stderr[-2000:]
        if p.returncode < 0 or "MemoryError" in p.stderr or "memory allocation" in p.stderr:
            res["status"] = "crash_or_memory"
    return res


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--worker", nargs=2)
    ap.add_argument("--sizes-only", action="store_true")
    ap.add_argument("--set", nargs="+", default=["netlib"])
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--methods", nargs="+", default=list(METHODS))
    ap.add_argument("--out", type=Path)
    ap.add_argument("--time-limit", type=float, default=600.0)
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--single-run-above", type=float, default=60.0, help="1 回目がこれ以上かかったら 1 回だけ")
    ap.add_argument("--mem-gb", type=float, default=13.5)
    ap.add_argument("--sizes", type=Path, help="前処理後の大きさの JSON (--min-presolved-rows で使う)")
    ap.add_argument("--min-presolved-rows", type=int, default=0, help="Kennington・Mittelmann だけ、前処理後の行数がこれ未満の問題を除く")
    args = ap.parse_args()
    if args.worker:
        worker(args.worker[0], args.worker[1], args.sizes_only)
        return

    ref = reference_objs()
    done = set()
    if args.out and args.out.exists():
        for line in args.out.read_text().splitlines():
            r = json.loads(line)
            done.add((r["set"], r["problem"], r["method"]))
    sizes = json.loads(args.sizes.read_text()) if args.sizes else {}
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
    for set_name in args.set:
        for name in problems(set_name):
            if args.only and name not in args.only:
                continue
            key = f"{set_name}/{name}"
            if args.min_presolved_rows and set_name != "netlib" and sizes.get(key, {}).get("n_rows_out", 0) < args.min_presolved_rows:
                continue
            todo = [m for m in args.methods if (set_name, name, m) not in done]
            if not todo and not args.sizes_only:
                continue
            workdir = Path(tempfile.mkdtemp(prefix="xbench_", dir=str(REPO / ".mittelmann_cache"))) if set_name == "mittelmann" else None
            try:
                mps = materialize(set_name, name, workdir) if workdir else materialize(set_name, name, Path("."))
                if args.sizes_only:
                    r = run_one(mps, "simplex", args.time_limit, args.mem_gb, sizes_only=True)
                    print(json.dumps({"key": key, **r.get("presolve_size", {})}), flush=True)
                    continue
                for method in todo:
                    runs = []
                    for rep in range(args.reps):
                        r = run_one(mps, METHODS[method], args.time_limit, args.mem_gb)
                        runs.append(r)
                        if r["status"] != "optimal" or r["time"] >= args.single_run_above:
                            break
                    times = sorted(r["time"] for r in runs)
                    med = runs[[r["time"] for r in runs].index(times[len(times) // 2])]
                    rv = ref.get((set_name, name))
                    obj = med.get("obj")
                    ok = med["status"] == "optimal" and obj is not None and (rv is None or abs(obj - rv) <= 1e-6 * max(1.0, abs(rv)))
                    rec = {"set": set_name, "problem": name, "method": method, "status": med["status"],
                           "time": med["time"], "times": [r["time"] for r in runs], "obj": obj, "ref_obj": rv,
                           "solved": ok, "events": med.get("events"), "stderr": med.get("stderr")}
                    print(f"{key:30s} {method:16s} {med['status']:10s} {med['time']:9.3f}s ok={ok} obj={obj} ref={rv}", flush=True)
                    if args.out:
                        with open(args.out, "a") as f:
                            f.write(json.dumps(rec) + "\n")
            finally:
                if workdir:
                    shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    main()
