"""ログの FRAG 行 (ENOMOTO_MIP_FRAG_DIAG) を表にする。使い方: python frag.py ログ [ログ ...]"""
import re, sys
pat = re.compile(r'FRAG: t ([\d.]+) tid (\d+) node (\d+) label "([^"]*)" nfree_int (\d+) hash (\w+) inc (\S+) status (\w+) nodes (\d+) obj (\S+) secs ([\d.]+)')
for f in sys.argv[1:]:
    print("==", f)
    tot = 0.0
    for l in open(f):
        m = pat.search(l)
        if m:
            t, tid, node, lab, nf, h, inc, st, nn, obj, secs = m.groups()
            tot += float(secs)
            print(f"{float(t):7.2f} tid{tid} n{node:>5} {lab:22s} free {nf:>4} inc {inc:>6} {st:10s} nodes {nn:>4} obj {obj:14s} {float(secs):5.2f}s  (sum {tot:6.2f}s) {h}")
        elif "new incumbent" in l:
            print("   ", l.strip()[5:120])
