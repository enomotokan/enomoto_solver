"""PRBIMP: CLQPRB の h の試し。python hscreen.py 設定 問題 種 [環境変数=値 ...]。600 秒の設定で回し、根の LP の大きさの行が出たら止めて、
CLQPRB=h の行を出す (h の sub-MIP は前処理の中で解くので、根の LP の前に終わる)。"""
import os, subprocess, sys, threading
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
name, p, seed = sys.argv[1], sys.argv[2], sys.argv[3]
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CFGS[name]); env.update(dict(kv.split("=", 1) for kv in sys.argv[4:]))
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=seed, ENOMOTO_MIP_LOG="1")
pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
killer = threading.Timer(300, pr.kill); killer.start()
for line in pr.stdout:
    if "CLQPRB=h" in line or "CLQPRB=g" in line or "CLQPRB=r" in line or "start solution" in line:
        print(p, seed, " ".join(sys.argv[4:]), line.strip()[:260], flush=True)
    if line.startswith("MIP: root LP size"):
        break
killer.cancel(); pr.kill(); pr.wait()
