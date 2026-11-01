"""neos-1171448 を real と real+d2lp1 で交互に各 6 回解く (4 スレッド・60 秒、ENOMOTO_MIP_LOG=1、2026-11-01)。
ログは <設定>_<回>.log (stderr + stdout)。使い方: python run.py。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
CFGS = {
    "real": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    "real_d2lp1": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
}
for rep in range(6):
    for c in (list(CFGS) if rep % 2 == 0 else list(CFGS)[::-1]):
        out = D / f"{c}_{rep}.log"
        if out.exists():
            continue
        env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / "neos-1171448.mps"),
                            "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        out.write_text(r.stderr + "\n" + r.stdout)
        print(out.name, r.stdout.strip().splitlines()[-1][:200] if r.stdout.strip() else "NO OUTPUT", flush=True)
