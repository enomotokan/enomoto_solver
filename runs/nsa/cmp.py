"""40 問 + 16 問の候補の結果を比べる: python runs/nsa/cmp.py <SHA> <名前:回> ..."""
import json, math, sys
from pathlib import Path
B = Path(__file__).resolve().parents[2] / 'benchmarks'
sha = sys.argv[1]
def solved(r): return r and r.get('status') == 'optimal'
def sgm(ts, s=10.0): return math.exp(sum(math.log(t + s) for t in ts) / len(ts)) - s
res = {}
for a in sys.argv[2:]:
    n, r = a.split(':')
    for suite in ('miplib_results', 'highsonly16'):
        d = json.load(open(B / f'{suite}_4t_60s_20261020_{sha}_{n}_r{r}.json'))
        res.setdefault(suite, {})[a] = {k: v['ours'] for k, v in d.items()}
for suite, runs in res.items():
    print(f'## {suite}')
    keys = list(next(iter(runs.values())))
    for a, d in runs.items():
        ts = [min(d[k]['time'], 60.0) if solved(d[k]) else 60.0 for k in keys]
        print(f'{a}: solved {sum(solved(d[k]) for k in keys)}/{len(keys)} SGM {sgm(ts):.2f}')
    # 不一致: 両方 optimal で目的値が違う
    for k in keys:
        rs = [runs[a][k] for a in runs]
        objs = [r['objective'] for r in rs if solved(r)]
        if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])):
            print('MISMATCH', k, objs)
    print('| 問題 | ' + ' | '.join(runs) + ' |')
    for k in keys:
        cells = []
        for a in runs:
            r = runs[a][k]
            cells.append(f"{r['time']:.1f}" if solved(r) else f"TL {r.get('bound') or 0:.6g}/{r.get('objective') or 0:.6g}")
        ss = [solved(runs[a][k]) for a in runs]
        tt = [runs[a][k]['time'] for a in runs if solved(runs[a][k])]
        if len(set(ss)) > 1 or (tt and max(tt) - min(tt) > max(3, 0.2 * min(tt))) or len(sys.argv) > 99:
            print(f'| {k} | ' + ' | '.join(cells) + ' |')
