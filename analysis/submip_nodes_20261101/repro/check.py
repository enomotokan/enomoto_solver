"""再スタート 2 の問題 (h80x6320d_restart2.mps.gz) を HiGHS とこのクレートで解き比べる (3.4 節)。
使い方: python check.py (このクレートは run_miplib_benchmark.py の --worker-ours で、前処理あり・なしの 2 通り)。
HiGHS と前処理なしは 6382.099 で最適、前処理ありは 0.02 秒で infeasible を返す (2026-11-01、`54cf99cb` の上)。"""
import gzip, os, shutil, subprocess, sys, tempfile
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[2]
tmp = Path(tempfile.mkdtemp()) / "h80_r2.mps"
with gzip.open(D / "h80x6320d_restart2.mps.gz", "rb") as s, open(tmp, "wb") as d:
    shutil.copyfileobj(s, d)
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("time_limit", 120.0); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(str(tmp)); h.run()
print("HiGHS:", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value)
for extra in ({}, {"ENOMOTO_MIP_NO_PRESOLVE": "1"}):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", **extra)
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(tmp), "--time-limit", "60", "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True)
    print("enomoto", extra or "(既定)", r.stdout.strip().splitlines()[-1][:120])
