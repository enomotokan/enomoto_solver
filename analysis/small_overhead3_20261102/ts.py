"""ログの各行に、solve の呼び出しからの経過時刻 (壁時計) を付けて 1 回解く (内訳の表のため)。使い方: python ts.py <問題> <出力> [KEY=VAL ...]"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb  # noqa: E402
mps = rb.fetch(sys.argv[1], WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", PYTHONUNBUFFERED="1", **dict(a.split("=", 1) for a in sys.argv[3:]))
code = ("import sys,time;from pathlib import Path;from enomoto_solver import benchmark_highs as bh;_,lp=bh._load_lp(Path(sys.argv[1]));"
        "m,_,_=bh._build_our_model(lp);sys.stderr.write('@@START\\n');sys.stderr.flush();t=time.perf_counter();r=m.solve(None,False,60.0,None,None,4);"
        "sys.stderr.write('@@END %.3f %s %s\\n'%(time.perf_counter()-t,r['status'],r['objective']))")
p = subprocess.Popen([sys.executable, "-c", code, str(mps)], env=env, stderr=subprocess.PIPE, text=True, bufsize=1)
t0 = None; out = []
for l in p.stderr:
    now = time.perf_counter()
    if l.startswith("@@START"): t0 = now; continue
    out.append(f"[{(now - t0) if t0 else 0:7.3f}] {l}")
p.wait(); Path(sys.argv[2]).write_text("".join(out)); print(out[-1].strip())
