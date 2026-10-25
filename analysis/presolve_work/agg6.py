"""6 章の集計: benchmarks/cand_logs/<設定>_r<回>.{40,16}.log から、問題ごとの最適の数・時間の平均と標準偏差・目的値の
ばらつきを出す。HiGHS の最適値と違う「最適」は MISMATCH と出す。使い方: python3 agg6.py pwdef pwwork"""
import sys, re, glob, statistics as st, collections
R = collections.defaultdict(lambda: collections.defaultdict(list))
mism = []
for cfg in sys.argv[1:]:
    for f in sorted(glob.glob(f'../../benchmarks/cand_logs/{cfg}_r*.log')):
        for l in open(f):
            m = re.match(r'(\S+)\s+highs=\s*(\S+) (\S+) obj=(\S+)\s+ours=\s*(\S+) (\S+) obj=(\S+)', l)
            if not m:
                continue
            p, hs, ht, ho, os_, ot, oo = m.groups()
            oo = None if oo == 'None' else float(oo)
            ho = None if ho == 'None' else float(ho)
            R[p][cfg].append((os_, float(ot), oo))
            if os_ == 'optimal' and hs == 'Optimal' and ho is not None and oo is not None and abs(oo - ho) > 1e-6 * max(1, abs(ho)):
                mism.append((cfg, f, p, oo, ho))
cfgs = sys.argv[1:]
print('| 問題 | ' + ' | '.join(f'{c} 最適 | {c} 時間 平均 (sd) | {c} 目的値 (sd)' for c in cfgs) + ' |')
print('|---|' + '---|---|---|' * len(cfgs))
tot = {c: [0, 0, [], []] for c in cfgs}
for p in sorted(R):
    cells = []
    for c in cfgs:
        v = R[p].get(c, [])
        n_opt = sum(1 for s, _, _ in v if s == 'optimal')
        ts = [t for _, t, _ in v]
        os_ = [o for _, _, o in v if o is not None]
        tsd = st.pstdev(ts) if len(ts) > 1 else 0.0
        osd = st.pstdev(os_) if len(os_) > 1 else 0.0
        tot[c][0] += n_opt; tot[c][1] += len(v); tot[c][2] += ts; tot[c][3].append(tsd)
        cells += [f'{n_opt}/{len(v)}', f'{st.mean(ts):.1f} ({tsd:.1f})' if ts else '', f'{st.mean(os_):.6g} ({osd:.3g})' if os_ else '—']
    print(f'| {p} | ' + ' | '.join(cells) + ' |')
for c in cfgs:
    o, n, ts, sds = tot[c]
    print(f'\n{c}: 最適 {o}/{n}、時間の平均 {st.mean(ts):.2f} 秒、問題ごとの時間の標準偏差の平均 {st.mean(sds):.2f} 秒')
for x in mism:
    print('MISMATCH', *x)
