"""MPS を ENOMOTO で解く。使い方: python solve_en.py <mps> <スレッド> <秒>"""
import sys, time, json
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, th, T = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, th, log=True)
print('RESULT', json.dumps({'status': r['status'], 'obj': r['objective'], 'bound': r['best_bound'], 'nodes': r['nodes'], 'time': time.perf_counter() - t}))
