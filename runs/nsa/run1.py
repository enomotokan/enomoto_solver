"""1 問を解いて runs/nsa/results.tsv に 1 行足す (4 スレッド・60 秒、log=True、ENOMOTO_MIP_LOG=1)。
使い方: python runs/nsa/run1.py <問題> <名前> <回> [ENV=VAL ...]"""
import os, re, subprocess, sys, json
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
prob, name, rep = sys.argv[1:4]
extra = dict(a.split('=', 1) for a in sys.argv[4:])
sys.path.insert(0, str(REPO / 'scripts'))
from run_miplib_benchmark import fetch
mps = fetch(prob, REPO / '.miplib_cache')
code = f"""
import time, json
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, 60, None, None, 4, log=True)
t = time.perf_counter() - t
print('RESULT ' + json.dumps({{'status': r['status'], 'obj': r['objective'], 'bound': r['best_bound'], 'nodes': r['nodes'], 'time': t}}))
"""
env = dict(os.environ, ENOMOTO_MIP_THREADS='4', ENOMOTO_MIP_LOG='1', **extra)
p = subprocess.run([sys.executable, '-c', code], capture_output=True, text=True, env=env, timeout=200)
log = REPO / 'runs' / 'nsa' / 'logs' / f'{prob}_{name}_{rep}.log'
out = p.stdout + p.stderr
res = next((json.loads(l[7:]) for l in p.stdout.splitlines() if l.startswith('RESULT ')), {})
diag = [l for l in out.splitlines() if l.startswith('MIP: node diag: ')]
d = {}
if diag:
    m = re.search(r'nodes (\d+) node LPs (\d+) mean rows ([\d.]+) iters (\d+) \(([\d.]+)/LP\) LP secs ([\d.]+) sep rounds (\d+) added (\d+) node cut secs ([\d.]+) final rows (\d+)', diag[-1])
    if m:
        d = dict(zip('mnodes lps rows iters itlp lpsecs seprounds added sepsecs finalrows'.split(), m.groups()))
rest = [l for l in out.splitlines() if 'first LP iterations after a restore' in l]
rs = re.search(r'basis restores (\d+) \(basic count mismatch (\d+)\), first LP iterations after a restore (\d+)', rest[-1]) if rest else None
fl = f"{int(rs.group(3))/max(1,int(rs.group(1))):.0f}" if rs else ''
nca = [l for l in out.splitlines() if l.startswith('MIP: adaptive node cuts')]
rate = re.search(r'rate ([\d.]+)', nca[-1]).group(1) if nca else ''
# 解けた時刻 (ソルバーの中) は RESULT の time
row = [prob, name, rep, res.get('status', 'crash'), f"{res.get('time', 0):.1f}", f"{res.get('bound') or 0:.6g}", f"{res.get('obj') or 0:.6g}", str(res.get('nodes', '')),
       d.get('mnodes', ''), d.get('itlp', ''), d.get('rows', ''), d.get('finalrows', ''), d.get('seprounds', ''), d.get('sepsecs', ''), d.get('lpsecs', ''), fl, rate, ' '.join(sys.argv[4:])]
with open(REPO / 'runs' / 'nsa' / 'results.tsv', 'a') as f:
    f.write('\t'.join(row) + '\n')
log.write_text('\n'.join(l for l in out.splitlines() if l.startswith(('MIP:', 'RESULT')) or re.match(r'^\s*[\d.]+s?\s', l) or 'nodes' in l[:80])[-200000:])
print('\t'.join(row), flush=True)
