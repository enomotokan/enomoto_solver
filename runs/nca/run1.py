"""1 問を 4 スレッド・60 秒で解き、ログ (stderr) を runs/nca/logs/<問題>_<名前>_<回>.log に、要約を runs/nca/results.tsv に足す。

使い方: python runs/nca/run1.py <問題> <名前> <回> [ENV=VAL ...]
"""
import json, os, re, subprocess, sys, time
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
prob, name, rep = sys.argv[1], sys.argv[2], sys.argv[3]
args = [a for a in sys.argv[4:] if a != 'NOLOG']
env = dict(os.environ, ENOMOTO_MIP_THREADS='4', **dict(a.split('=', 1) for a in args))
if 'NOLOG' not in sys.argv[4:]:  # NOLOG: ENOMOTO_MIP_LOG を付けない (診断の行は出ない)
    env['ENOMOTO_MIP_LOG'] = '1'
mps = next(p for p in (REPO / '.miplib_cache').glob(prob + '.mps*'))
code = f"""
import json, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r})); m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, 60.0, None, None, 4, log=True); t = time.perf_counter() - t
print(json.dumps(dict(status=r['status'], obj=r['objective'], bound=r['best_bound'], nodes=r['nodes'], time=t)))
"""
logd = REPO / 'runs' / 'nca' / 'logs'
logd.mkdir(exist_ok=True)
log = logd / f'{prob}_{name}_{rep}.log'
with open(log, 'w') as f:
    r = subprocess.run([sys.executable, '-c', code], cwd='/tmp', env=env, stdout=subprocess.PIPE, stderr=f, text=True, timeout=200)
res = json.loads(r.stdout.strip().splitlines()[-1])
txt = log.read_text()
def grab(pat):
    m = re.findall(pat, txt)
    return m[-1] if m else '-'
diag = grab(r'MIP: node diag: (.*)')
nca = grab(r'MIP: adaptive node cuts: (.*)')
line = f"{prob}\t{name}\t{rep}\t{res['status']}\t{res['time']:.1f}\t{res['nodes']}\t{res['obj']}\t{res['bound']}\t{diag}\t{nca}"
with open(REPO / 'runs' / 'nca' / 'results.tsv', 'a') as f:
    f.write(line + '\n')
print(line, flush=True)
