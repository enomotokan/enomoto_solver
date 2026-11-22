"""b が働くか (外側の根で刻みの行を作るか) だけを見る: ENOMOTO_MIP_OBJSTEPCUT=b・1 スレッドで回し、最初の「OBJSTEP」の行 (働く)、
「cut round 1:」の行・根の終わり・制限時間 (働かない、または根に届かない) で止める。python acts.py 問題... > acts_M.txt"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_OBJSTEPCUT": "b", "ENOMOTO_MIP_OBJSTEPCUT_LOG": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1"})
for p in sys.argv[1:]:
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", "900", "--threads", "1"],
                          cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    t0 = time.time(); res = "根の分離に届かず (900 秒)"
    for line in pr.stderr:
        if line.startswith("OBJSTEP"): res = "働く: " + line.strip(); break
        if "cut round 1:" in line: res = "働かない (目的に刻みがない)"; break
        if time.time() - t0 > 900: break
    pr.kill(); pr.wait()
    print(p, "|", res, flush=True)
