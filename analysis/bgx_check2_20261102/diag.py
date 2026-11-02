"""3. 診断: 写しのログ (`ENOMOTO_MIP_BG_CUTS_LOG=1`、写しの求解を verbose にする) を付けて既定 (def) と `ENOMOTO_MIP_BG_CUTS_BASIS_X=0` (x0) を 1 回ずつ解く。
引数: 問題 (カンマ区切り)。ログ diag/{問題}_{形}.log"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "x0": {"ENOMOTO_MIP_BG_CUTS_BASIS_X": "0"}, "def_dse": {"ENOMOTO_MIP_CUT_DSE_EXACT": "1"},
        "x0_dse": {"ENOMOTO_MIP_BG_CUTS_BASIS_X": "0", "ENOMOTO_MIP_CUT_DSE_EXACT": "1"}}
# 第 2 引数で形を選ぶ (カンマ区切り、既定 def,x0)。def_dse・x0_dse は既存の試作 `ENOMOTO_MIP_CUT_DSE_EXACT` (カットを足した LP を出発の基底の厳密な DSE 重みから解く) を足す
SEL = sys.argv[2].split(",") if len(sys.argv) > 2 else ["def", "x0"]
CFGS = {k: CFGS[k] for k in SEL}
for p in sys.argv[1].split(","):
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CFGS.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", **e)
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=400)
        (D / "diag" / f"{p}_{c}.log").write_text(r.stderr + r.stdout)
        print(p, c, r.stdout.strip().splitlines()[-1][:150], flush=True)
