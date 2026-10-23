# HiGHS の計装ログ (HSEPT 行、根の分離のラウンドごと) の段階ごとの平均 (ms/ラウンド): python hsum.py logs/*_h.err
import sys
keys = ['impl', 'clq', 'prop', 'tab', 'path', 'modk', 'poolsel']
for f in sys.argv[1:]:
    rows = [l for l in open(f) if l.startswith('HSEPT')]
    if not rows:
        continue
    S = {k: 0.0 for k in keys}
    nc = 0
    for l in rows:
        t = l.split()
        for k in keys:
            S[k] += float(t[t.index(k) + 1])
        nc += int(t[t.index('ncut') + 1])
    n = len(rows)
    adds = [l for l in open(f) if l.startswith('HADD')]
    lpt = sum(float(l.split()[l.split().index('lptime') + 1]) for l in adds)
    sep = sum(S[k] for k in ['impl', 'clq', 'tab', 'path', 'modk'])
    print(f"{f}: rounds {n} sep {1e3*sep/n:.1f} = " + ' '.join(f"{k} {1e3*S[k]/n:.1f}" for k in keys) + f" | cuts to LP/round {nc/n:.1f} | LP after add {1e3*lpt/max(1,len(adds)):.1f} ({len(adds)} adds)")
