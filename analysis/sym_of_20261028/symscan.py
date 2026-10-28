"""問題ごとに短時間 (ENOMOTO_MIP_LOG=1) 解いて、対称性の検出の行を拾う。python symscan.py 時間 問題..."""
import sys, os, subprocess, re
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
REPO = Path('/home/user/enomoto_solver')
sys.path.insert(0, str(REPO / 'scripts'))
import run_miplib_benchmark as r
CACHE = REPO / '.miplib_cache'
tl = float(sys.argv[1])
probs = sys.argv[2:]
for n in probs:
    r.fetch(n, CACHE)

def one(n):
    code = ("import sys;from pathlib import Path;from enomoto_solver import benchmark_highs as bh;"
            f"_,lp=bh._load_lp(Path('{CACHE}/{n}.mps'));m,_,_=bh._build_our_model(lp);m.solve(None,False,{tl},None,None,1)")
    env = dict(os.environ, ENOMOTO_MIP_LOG='1')
    p = subprocess.run([sys.executable, '-c', code], capture_output=True, text=True, env=env, timeout=tl * 3 + 120)
    lines = [l for l in p.stderr.splitlines() if 'MIP: symmetry' in l or 'orbitopes' in l or 'orbital fixing' in l]
    return n, lines

with ThreadPoolExecutor(4) as ex:
    for n, lines in ex.map(one, probs):
        print(f"== {n}")
        for l in lines:
            print("   " + l[:220])
        sys.stdout.flush()
