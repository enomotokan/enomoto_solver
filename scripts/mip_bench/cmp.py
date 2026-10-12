import json, math, sys
from pathlib import Path
import os
S = Path(os.environ.get('MIP_BENCH_DIR', Path(__file__).parent / 'work'))
ref = json.load(open(S/'ref.json'))
def load1(t): return json.load(open(S/f'res_{t}.json')) if (S/f'res_{t}.json').exists() else json.load(open(S/f'{t}.json'))
def load(spec):
    # 'a+b' は複数回の計測: 問題ごとに log(t+10) を平均した時間 (状態は最後の回、解けた回が半分以上なら optimal 扱い)
    runs = [load1(t) for t in spec.split('+')]
    out = {}
    for n in runs[0]:
        if not all(n in r for r in runs): continue
        ts = [tm(r[n]) for r in runs]
        o = dict(runs[-1][n]); g = math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
        o['time'] = o['cpu'] = g
        o['status'] = 'optimal' if sum(r[n].get('status') == 'optimal' for r in runs) * 2 >= len(runs) and g < 89.9 else o.get('status')
        if o['status'] != 'optimal': o['time'] = 90.0
        o['nodes'] = '/'.join(str(r[n].get('nodes')) for r in runs)
        out[n] = o
    return out
KEY = 'time'
def tm(o): return o.get(KEY, o['time']) if o.get('status') == 'optimal' and o.get('time') is not None else 90.0
def sgm(d, names): return math.exp(sum(math.log(tm(d[n]) + 10) for n in names) / len(names)) - 10
args = sys.argv[1:]
if args[0] == '--cpu': KEY = 'cpu'; args = args[1:]
base, cands = args[0], args[1:]
b = load(base)
for c in cands:
    d = load(c); names = [n for n in b if n in d]
    sb, sc = sgm(b, names), sgm(d, names)
    wrong = []
    for n in names:
        o = d[n]
        if o.get('status') == 'optimal' and ref[n].get('status') == 'Optimal':
            e = abs(o['objective'] - ref[n]['ref']) / max(1, abs(ref[n]['ref']))
            if e > 1e-9: wrong.append((n, o['objective'], ref[n]['ref']))
        if str(o.get('status')).startswith(('crash', 'killed')): wrong.append((n, o['status'], None))
    ns = sum(d[n].get('status') == 'optimal' for n in names)
    print(f"{c:8s} n={len(names)} SGM {sb:.3f} -> {sc:.3f}  ({(sc/sb-1)*100:+.2f}%)  solved {ns}  wrong {wrong}")
    for n in names:
        tb, tc = tm(b[n]), tm(d[n])
        if abs(tc - tb) > max(1.0, 0.1 * tb): print(f"     {n:20s} {tb:6.1f} -> {tc:6.1f}  nodes {b[n].get('nodes')} -> {d[n].get('nodes')}")
