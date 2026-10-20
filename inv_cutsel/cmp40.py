"""40 問 (と 16 問) の結果 JSON の比較: 解けた数・SGM (shift 10)・HiGHS との不一致・設定ごとに時間が 1.5 倍 / 5 秒以上違う問題。
使い方: python inv_cutsel/cmp40.py <SHA> <名前_r回>..."""
import json, math, sys
from pathlib import Path
B = Path(__file__).resolve().parents[1] / 'benchmarks'
sha, runs = sys.argv[1], sys.argv[2:]
solved = lambda r: r.get('status') in ('Optimal', 'optimal')
for base in ('miplib_results', 'highsonly16'):
    data = {}
    for r in runs:
        f = B / f'{base}_4t_60s_20261020_{sha}_{r}.json'
        if f.exists():
            data[r] = json.load(open(f))
    if not data: continue
    names = list(next(iter(data.values())))
    print(f'## {base}')
    for r, d in data.items():
        ts = [d[n]['ours']['time'] if solved(d[n]['ours']) else 60.0 for n in names]
        sgm = math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
        wrong = [n for n in names if solved(d[n]['ours']) and solved(d[n]['highs']) and abs(d[n]['ours']['objective'] - d[n]['highs']['objective']) > 1e-4 * max(1, abs(d[n]['highs']['objective'])) + 1e-6]
        print(f'{r:20s} solved {sum(solved(d[n]["ours"]) for n in names):2d}/{len(names)} SGM {sgm:6.2f} mismatch {wrong}')
    print('| 問題 | ' + ' | '.join(data) + ' |')
    print('|---|' + '---|' * len(data))
    for n in names:
        v = [(d[n]['ours']['time'] if solved(d[n]['ours']) else None) for d in data.values()]
        tv = [x if x is not None else 60 for x in v]
        if max(tv) - min(tv) >= 5 and max(tv) >= 1.5 * min(tv):
            cells = [f'{x:.1f}' if x is not None else f't ({d[n]["ours"]["bound"]:.6g})' for x, d in zip(v, data.values())]
            print(f'| {n} | ' + ' | '.join(cells) + ' |')
