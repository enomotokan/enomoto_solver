"""4 問 (mzzv42z・neos-960392・h80x6320d・physiciansched6-2) を HiGHS と ENOMOTO でログつきで解く (2026-10-30、4 スレッド・60 秒)。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
PROBS = ["mzzv42z", "neos-960392", "h80x6320d", "physiciansched6-2"]
HS = """
import highspy, sys, time
h = highspy.Highs(); h.setOptionValue('threads', 4); h.setOptionValue('time_limit', 60.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); print('WALL', time.perf_counter() - t)
"""
reps = int(sys.argv[1]) if len(sys.argv) > 1 else 1
for rep in range(reps):
    for p in PROBS:
        mps = C / f"{p}.mps.gz"
        out = D / f"{p}_highs_{rep}.log"
        if not out.exists():
            r = subprocess.run([sys.executable, "-c", HS, str(mps)], capture_output=True, text=True, timeout=400)
            out.write_text(r.stdout + r.stderr)
        out = D / f"{p}_ours_{rep}.log"
        if not out.exists():
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4")
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            out.write_text(r.stderr + "\n" + r.stdout)
