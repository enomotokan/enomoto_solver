"""logs/q*.txt から 問題 × 設定 の表を作る。セル: 各回の「秒 (解けた) か 60 秒の暫定解」/ 根の下界。"""
import sys, collections, glob
rows = collections.defaultdict(list)
for f in sorted(glob.glob(sys.argv[1] if len(sys.argv) > 1 else 'logs/q*.txt')):
    for l in open(f):
        t = l.split()
        if len(t) < 10 or t[3] != 'RESULT':
            continue
        p, c, k, st, obj, bb, nodes, sec, root = t[0], t[1], t[2], t[4], t[5], t[6], t[7], t[8], t[9]
        rows[(p, c)].append((k, st, float(obj) if obj != 'None' else None, float(sec), float(root) if root != 'None' else None))
probs = list(dict.fromkeys(p for p, _ in rows))
cfgs = list(dict.fromkeys(c for _, c in rows))
OPT = {'mc11': 11689, 'neos-960392': -238, 'neos-911970': 54.76, 'mzzv42z': -20540, 'roll3000': 12890, 'noswot': -41}
def cell(v, p=None):
    out = []
    for k, st, obj, sec, root in sorted(v):
        wrong = st == 'optimal' and p in OPT and abs(obj - OPT[p]) > 1e-6 * max(1, abs(OPT[p]))
        out.append(f"誤 {obj:g} ({sec:.0f}s)" if wrong else f"**{sec:.0f}s**" if st == 'optimal' else f"{obj:g}")
    return " / ".join(out)
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    print(f"| {p} | " + " | ".join(cell(rows.get((p, c), []), p) for c in cfgs) + " |")
