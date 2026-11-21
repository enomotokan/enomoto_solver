"""neos-3988577-wolgan の dc (`ENOMOTO_MIP_DSE_COST=1`)・種 1 の再現 (README §7): python rep_wolgan.py 回
診断 `ENOMOTO_MIP_DSE_COST_LOG=1` で、作り直しを省いたか作ったかを求解ごとに出す (解き方は変えない)。4 スレッド・600 秒。
RW_CFG=def なら既定で回す。ログは logs_m/neos-3988577-wolgan_<設定>_s1_rep<回>.log.gz (経過秒つき)。"""
import gzip, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
k = sys.argv[1]
env = {kk: v for kk, v in os.environ.items() if not kk.startswith("ENOMOTO_")}
CFG = os.environ.get("RW_CFG", "dc")  # RW_CFG=def なら既定 (試作なし) で回す
env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_DSE_COST_LOG="1")
if CFG == "dc": env["ENOMOTO_MIP_DSE_COST"] = "1"
t0 = time.time()
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache/neos-3988577-wolgan.mps"), "--time-limit", "600", "--threads", "4"],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
with gzip.open(D / f"logs_m/neos-3988577-wolgan_{CFG}_s1_rep{k}.log.gz", "wt") as f:
    for l in pr.stderr: f.write(f"[{time.time() - t0:8.2f}] {l}")
print(pr.stdout.read().strip().splitlines()[-1:], flush=True)
