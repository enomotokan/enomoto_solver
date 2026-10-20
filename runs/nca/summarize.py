"""runs/nca/results.tsv を問題 × 設定の表にする (時間切れは「TL 下界」)。使い方: python runs/nca/summarize.py [設定 ...]"""
import sys
from collections import defaultdict
from pathlib import Path
rows = [l.rstrip('\n').split('\t') for l in open(Path(__file__).with_name('results.tsv'))]
cfgs = sys.argv[1:] or sorted({r[1] for r in rows})
tab = defaultdict(list)
for r in rows:
    prob, cfg, rep, st, t, nodes, obj, bd = r[:8]
    s = f'{float(t):.1f}' if st == 'optimal' else f'TL {float(bd):.4g}'
    tab[prob, cfg].append(f'{s} ({nodes})')
probs = list(dict.fromkeys(r[0] for r in rows))
print('| 問題 | ' + ' | '.join(cfgs) + ' |')
print('|---' * (len(cfgs) + 1) + '|')
for p in probs:
    print(f'| {p} | ' + ' | '.join('・'.join(tab[p, c]) or '-' for c in cfgs) + ' |')
