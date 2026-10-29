"""h80x6320d をログ付きで解き、早めの再スタートがどのヒューリスティクスの解で起きるかを見る (2026-10-29)。"""
import os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
REPO = Path(r"C:\Users\enomo\Desktop\2026\python\ENOMOTO-Solver")
PY = str(REPO / ".tmpx" / "bench_venv" / "Scripts" / "python.exe")
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb
D = Path(__file__).resolve().parent
CFGS = {"new": {}, "noshift": {"ENOMOTO_MIP_SHIFT_HEURS": "0"}, "skiplight": {"ENOMOTO_MIP_EARLY_RESTART_SKIP_LIGHT": "1"}}
mps = rb.fetch("h80x6320d", REPO / ".miplib_cache")
for rep in (0, 1):
    for c, e in CFGS.items():
        env = dict(os.environ, PYTHONUTF8="1", ENOMOTO_MIP_THREADS="16", ENOMOTO_MIP_LOG="1", PYTHONPATH=str(WT / "python"), **e)
        r = subprocess.run([PY, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "16"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        (D / f"{c}_{rep}.log").write_text(r.stderr + "\n" + r.stdout, encoding="utf-8")
(D / "done").write_text(time.strftime("%H:%M:%S"))
