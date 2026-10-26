"""sumbench.py <接頭辞> <形>...: benchmarks/cand_logs/<接頭辞><形>_r*.{16,40}.log から問題ごとに最適の数・時間 (最適でなければ 60 TL) を並べる。目的値の食い違いも出す"""
import sys, glob, re, collections, statistics
pre, forms = sys.argv[1], sys.argv[2:]
res = collections.defaultdict(lambda: collections.defaultdict(list)); mism = []
for f in forms:
    for fn in sorted(glob.glob(f'benchmarks/cand_logs/{pre}{f}_r*.*.log')):
        r = re.search(r'_r(\d+)\.', fn).group(1)
        for l in open(fn):
            m = re.match(r'(\S+)\s+highs=\s*(.*?) ([0-9.]+) obj=(\S+)\s+ours=\s*(\S+) ([0-9.]+) obj=(\S+)', l)
            if not m: continue
            p, hs, ht, ho, os_, ot, oo = m.groups()
            res[p][f].append((int(r), os_, float(ot), oo))
            if os_ == 'optimal' and hs == 'Optimal' and abs(float(oo) - float(ho)) > 1e-6 * max(1, abs(float(ho))): mism.append((p, f, r, oo, ho))
print('| 問題 | ' + ' | '.join(forms) + ' |'); print('|---' * (len(forms) + 1) + '|')
for p in res:
    cells = []
    for f in forms:
        v = sorted(res[p][f])
        opt = [t for _, s, t, _ in v if s == 'optimal']
        ts = [t if s == 'optimal' else 60.0 for _, s, t, _ in v]
        txt = ' '.join(f'{t:.1f}' if s == 'optimal' else f'TL({float(o):g})' for _, s, t, o in v)
        cells.append(f'{len(opt)}/{len(v)} 中央 {statistics.median(ts):.1f}: {txt}' if v else '-')
    print(f'| {p} | ' + ' | '.join(cells) + ' |')
print('目的値の食い違い:', mism or 'なし')
