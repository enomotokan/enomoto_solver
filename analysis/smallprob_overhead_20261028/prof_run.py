"""小さい問題の段階別の時間の調査用: ENOMOTO・HiGHS を 1 回解き、時刻つきのログを残す。

使い方:
  python prof_run.py ours  <mps> <log> [--threads 4] [--time-limit 60]   (ENOMOTO_MIP_LOG=1・ENOMOTO_PROF_TREE_PAR=1 をつける)
  python prof_run.py highs <mps> <log> [--threads 4] [--time-limit 60]   (HiGHS の output_flag=True・log_file=<log>、parallel=on)

ours のログは各行の頭に「求解の始まり (`m.solve` の直前) からの経過秒」を [  1.234] の形でつける (ENOMOTO の標準エラーの
行は子プロセスから 1 行ずつ読む)。最後の行に結果の JSON を書く。
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def child_ours(mps: str, time_limit: float, threads: int) -> None:
    from enomoto_solver import benchmark_highs as bh

    _, lp = bh._load_lp(Path(mps))
    m, _, _ = bh._build_our_model(lp)
    sys.stderr.write("@@SOLVE_START\n")
    sys.stderr.flush()
    t = time.perf_counter()
    r = m.solve(None, False, time_limit, None, None, threads)
    t = time.perf_counter() - t
    sys.stderr.write("@@SOLVE_END\n")
    sys.stderr.flush()
    print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"],
                      "nodes": r["nodes"], "time": t}))


def run_ours(mps: str, log: Path, time_limit: float, threads: int) -> dict:
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_PROF_TREE_PAR="1", ENOMOTO_MIP_THREADS=str(threads))
    cmd = [sys.executable, __file__, "child-ours", mps, "-", "--threads", str(threads), "--time-limit", str(time_limit)]
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env, bufsize=1)
    t0 = None
    with open(log, "w", encoding="utf-8") as f:
        for line in p.stderr:
            now = time.perf_counter()
            if line.startswith("@@SOLVE_START"):
                t0 = now
            f.write(f"[{now - t0 if t0 is not None else 0.0:8.3f}] {line}")
        out = p.stdout.read()
        p.wait()
        r = None
        for l in reversed(out.splitlines()):
            if l.startswith("{"):
                r = json.loads(l)
                break
        f.write("@@RESULT " + json.dumps(r) + "\n")
    return r


def run_highs(mps: str, log: Path, time_limit: float, threads: int) -> dict:
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", True)
    h.setOptionValue("log_to_console", False)
    h.setOptionValue("log_file", str(log))
    h.setOptionValue("time_limit", time_limit)
    h.setOptionValue("threads", threads)
    h.setOptionValue("mip_rel_gap", 0.0)
    h.setOptionValue("parallel", "on")
    h.readModel(mps)
    t = time.perf_counter()
    h.run()
    t = time.perf_counter() - t
    info = h.getInfo()
    r = {"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value,
         "bound": info.mip_dual_bound, "nodes": info.mip_node_count, "time": t}
    with open(log, "a", encoding="utf-8") as f:
        f.write("@@RESULT " + json.dumps(r) + "\n")
    return r


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("kind", choices=["ours", "highs", "child-ours"])
    ap.add_argument("mps")
    ap.add_argument("log")
    ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--time-limit", type=float, default=60.0)
    a = ap.parse_args()
    if a.kind == "child-ours":
        return child_ours(a.mps, a.time_limit, a.threads)
    f = run_ours if a.kind == "ours" else run_highs
    r = f(a.mps, Path(a.log), a.time_limit, a.threads)
    print(json.dumps(r))


if __name__ == "__main__":
    main()
