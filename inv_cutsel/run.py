"""ENOMOTO を 4 スレッド・T 秒で 1 問解き、利用者向けログに経過時間を付けて保存する。
使い方: python inv_cutsel/run.py <問題> <ラベル> <秒> [ENV=VAL ...]  -> inv_cutsel/logs/<問題>_<ラベル>.log"""
import os, subprocess, sys, time, json
from pathlib import Path
REPO = Path(__file__).resolve().parents[1]
prob, label, T = sys.argv[1], sys.argv[2], float(sys.argv[3])
env = dict(os.environ, ENOMOTO_MIP_THREADS='4'); env.update(a.split('=', 1) for a in sys.argv[4:])
mps = REPO / '.miplib_cache' / f'{prob}.mps'
if not mps.exists():
    sys.path.insert(0, str(REPO / 'scripts'))
    from run_miplib_benchmark import fetch
    fetch(prob, REPO / '.miplib_cache')
child = f"""
import json, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, {T}, None, None, 4, log=True)
t = time.perf_counter() - t
print('RESULT ' + json.dumps({{'status': r['status'], 'objective': r['objective'], 'bound': r['best_bound'], 'nodes': r['nodes'], 'time': t, 'root_bound': r.get('root_bound')}}), flush=True)
"""
out = REPO / 'inv_cutsel' / 'logs' / f'{prob}_{label}.log'
t0 = time.time()
p = subprocess.Popen([sys.executable, '-c', child], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd='/tmp')
with open(out, 'w') as f:
    f.write(f'# {prob} {label} {T} {" ".join(sys.argv[4:])}\n')
    for line in p.stdout:
        f.write(f'[{time.time() - t0:7.2f}] {line}')
p.wait()
print(out)
