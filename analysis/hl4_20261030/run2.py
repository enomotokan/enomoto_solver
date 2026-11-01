"""切り分け (2026-10-30、4 スレッド・60 秒): 最適値を打ち切り値で与えた証明だけの時間 (`ENOMOTO_MIP_CUTOFF`) と、
physiciansched6-2 の根のカットの時間の上限を広げた回 (`ENOMOTO_T_MIP_CUT_TIME_FRAC`)。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
OPT = {"mzzv42z": -20540 + 0.5, "neos-960392": -238 + 0.5, "h80x6320d": 6382.09905 * (1 + 1e-7), "physiciansched6-2": 49324 + 0.5}
RUNS = [(p, "cutoff", {"ENOMOTO_MIP_CUTOFF": repr(v)}) for p, v in OPT.items()]
RUNS += [("physiciansched6-2", "cutfrac04", {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"}),
         ("physiciansched6-2", "cutfrac04_cutoff", {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4", "ENOMOTO_MIP_CUTOFF": repr(OPT["physiciansched6-2"])})]
for p, tag, e in RUNS:
    out = D / f"{p}_{tag}_0.log"
    if out.exists():
        continue
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **e)
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps.gz"),
                        "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
    out.write_text(r.stderr + "\n" + r.stdout)
