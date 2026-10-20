"""既定と試作の 2 つの結果 JSON (bench_cand_20261020.py の出力) を問題ごとに並べる。
使い方: python compare.py <既定.json> <試作.json> [...] (2 つずつ組にする)。主ギャップの基準は HiGHS と両者の最良値。"""
import json, math, sys
sys.path.insert(0, __import__('os').path.join(__import__('os').path.dirname(__file__), '..', '..', 'scripts'))
from run_miplib_benchmark import primal_gap, solved
fs = sys.argv[1:]
ds = [json.load(open(f)) for f in fs]
def t(r): return r['time'] if solved(r) and r.get('time') is not None else 60.0
def sgm(ts, s=10.0): return math.exp(sum(math.log(x + s) for x in ts) / len(ts)) - s
names = list(ds[0])
print('| 問題 | ' + ' | '.join(f.split('_')[-2] + '_' + f.split('_')[-1].replace('.json', '') for f in fs) + ' |')
tot = [[] for _ in ds]; gaps = [0.0] * len(ds); ns = [0] * len(ds)
for n in names:
    rs = [d[n]['ours'] for d in ds]
    h = ds[0][n]['highs']
    objs = [r.get('objective') for r in rs] + [h.get('objective')]
    cand = [o for o in objs if o is not None]
    # 最小化か最大化かは分からないので、HiGHS の値を基準にする (無ければ最初の値)
    ref = h.get('objective') if h.get('objective') is not None else (cand[0] if cand else None)
    cells = []
    for i, r in enumerate(rs):
        tot[i].append(t(r)); g = primal_gap(r.get('objective'), ref); gaps[i] += g; ns[i] += solved(r)
        cells.append((f"{r['time']:.1f}s" if solved(r) else f"t {r.get('objective')}"))
    if len(set(cells)) > 1 or any(not solved(r) for r in rs):
        print(f'| {n} | ' + ' | '.join(cells) + ' |')
print('| 解けた数 | ' + ' | '.join(str(x) for x in ns) + ' |')
print('| SGM (10 秒ずらし、未解決 60) | ' + ' | '.join(f'{sgm(x):.2f}' for x in tot) + ' |')
print('| 主ギャップの和 (HiGHS の値基準) | ' + ' | '.join(f'{x:.3f}' for x in gaps) + ' |')
