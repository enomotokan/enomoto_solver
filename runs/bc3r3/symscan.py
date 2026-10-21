"""40 問で対称性の検出の行を集める (4 スレッド、時間上限 2 秒)。使い方: python symscan.py [ENV=VAL ...]"""
import os, subprocess, sys
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'scripts'))
from run_miplib_benchmark import PROBLEMS
extra = dict(a.split('=', 1) for a in sys.argv[1:])
for n in PROBLEMS:
    mps = REPO / '.miplib_cache' / f'{n}.mps'
    code = f"from pathlib import Path\nfrom enomoto_solver import benchmark_highs as bh\n_, lp = bh._load_lp(Path({str(mps)!r}))\nm, _, _ = bh._build_our_model(lp)\nm.solve(None, False, 2, None, None, 4, log=True)"
    env = dict(os.environ, ENOMOTO_MIP_THREADS='4', ENOMOTO_MIP_LOG='1', **extra)
    p = subprocess.run([sys.executable, '-c', code], capture_output=True, text=True, env=env, timeout=300, cwd='/tmp')
    s = [l for l in (p.stdout + p.stderr).splitlines() if 'symmetry:' in l]
    print(n, ' | '.join(s) if s else '(no symmetry line)', flush=True)
