"""問題 (PROB、既定 beasleyC3) を 1 回解き、全ログを runs/bc3t16/logs/<名前>_<回>.log に保存する (スレッド数は TH、既定 16)。
使い方: python runs/bc3t16/run.py <名前> <回> <秒> [ENV=VAL ...]"""
import os, subprocess, sys, json
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
name, rep, T = sys.argv[1:4]
extra = dict(a.split('=', 1) for a in sys.argv[4:])
TH = int(os.environ.get('TH', '16'))
prob = os.environ.get('PROB', 'beasleyC3')
mps = REPO / '.miplib_cache' / (prob + '.mps.gz')
code = f"""
import time, json
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, {T}, None, None, {TH}, log=True)
t = time.perf_counter() - t
print('RESULT ' + json.dumps({{'status': r['status'], 'obj': r['objective'], 'bound': r['best_bound'], 'nodes': r['nodes'], 'time': t}}), flush=True)
"""
env = dict(os.environ, ENOMOTO_MIP_THREADS=str(TH), **extra)
env.setdefault('ENOMOTO_MIP_LOG', '1')
log = REPO / 'runs' / 'bc3t16' / 'logs' / (f'{name}_{rep}.log' if prob == 'beasleyC3' else f'{prob}_{name}_{rep}.log')
with open(log, 'w') as f:
    subprocess.run([sys.executable, '-c', code], stdout=f, stderr=subprocess.STDOUT, env=env, timeout=int(T) + 120, cwd='/tmp')
print(prob, name, rep, [l for l in log.read_text().splitlines() if l.startswith('RESULT')][-1:])
