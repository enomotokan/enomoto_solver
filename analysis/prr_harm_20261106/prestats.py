"""pre/<問題>.log (prr、前処理だけ) から、かけ直しの直前のプロービングの固定・締めた境界の数と、かけ直しの前後の行・自由な列を取り出し、
bench28 (def 対 prr、58 問 rep 40・41・117) の時間の比と並べる。"""
import re, json, glob, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for f in glob.glob(str(D.parents[0] / 'halve_submip_20261030/bench28_s*.jsonl')):
    for l in open(f):
        d = json.loads(l)
        if d['cfg'] in ('def', 'prr'):
            r = d['result']; t = r['time'] if r['status'] == 'optimal' else 60.0
            R[d['problem']].setdefault(d['cfg'], []).append(t)
rows = []
for p in open(D / 'pre' / (sys.argv[1] if len(sys.argv) > 1 else 'list.txt')).read().split():
    L = open(D / f'pre/{p}.log').read().splitlines()
    probe = None; pre = None; rec = []; pend = None
    for l in L:
        m = re.search(r'probing: \d+ of \d+ binaries probed .*?, (\d+) fixed, (\d+) bounds tightened', l)
        if m: probe = (int(m[1]), int(m[2]))
        m = re.search(r'presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+)', l)
        if m:
            if pend is not None:
                rec.append(pend + (int(m[2]), int(m[4]))); pend = None
            pre = (int(m[2]), int(m[4]))
        if 'presolve again after probing' in l and probe and pre:
            pend = (probe[0], probe[1], pre[0], pre[1])
    g = lambda c: (sum(R[p].get(c, [])) / max(1, len(R[p].get(c, [])))) if R[p].get(c) else float('nan')
    a, b = g('def'), g('prr')
    if rec:
        fx, tg, m0, f0, m1, f1 = rec[0]
        rows.append((p, fx, tg, m0, m1, f0, f1, len(rec), a, b))
    else:
        rows.append((p, None, None, None, None, None, None, 0, a, b))
print(f"{'問題':24} {'固定':>6} {'締め':>6} {'行前':>7} {'行後':>7} {'行%':>6} {'列%':>6} {'回':>2} {'def':>6} {'prr':>6} 比")
for p, fx, tg, m0, m1, f0, f1, k, a, b in sorted(rows, key=lambda r: -1 if r[1] is None else (r[1] / max(1, r[5]))):
    ratio = (b + 10) / (a + 10)
    if fx is None:
        print(f"{p:24} {'-':>6} {'-':>6} {'':>7} {'':>7} {'':>6} {'':>6} {0:>2} {a:6.1f} {b:6.1f} {ratio:.2f}")
    else:
        print(f"{p:24} {fx:6d} {tg:6d} {m0:7d} {m1:7d} {100*(m0-m1)/max(1,m0):6.1f} {100*fx/max(1,f0+fx):6.1f} {k:2d} {a:6.1f} {b:6.1f} {ratio:.2f}")
