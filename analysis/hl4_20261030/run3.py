"""physiciansched6-2: 根のカットの LP が 1 回の時間の上限で止まってもカットを外さない (`ENOMOTO_MIP_CUT_LP_CONTINUE`) と、
根のカットの時間の上限 (`ENOMOTO_T_MIP_CUT_TIME_FRAC`) の組 (2026-10-30、4 スレッド・60 秒)。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
RUNS = [("physiciansched6-2", "lpcont", {"ENOMOTO_MIP_CUT_LP_CONTINUE": "1"}),
        ("physiciansched6-2", "lpcont_frac04", {"ENOMOTO_MIP_CUT_LP_CONTINUE": "1", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"}),
        ("physiciansched6-2", "lpfrac01_frac04", {"ENOMOTO_T_MIP_CUT_LP_TIME_FRAC": "0.1", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"}),
        ("mzzv42z", "frac04", {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"})]
for p, tag, e in RUNS:
    out = D / f"{p}_{tag}_0.log"
    if out.exists():
        continue
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **e)
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps.gz"),
                        "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
    out.write_text(r.stderr + "\n" + r.stdout)
