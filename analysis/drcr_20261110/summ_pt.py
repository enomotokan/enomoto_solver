"""速い細分化 (既定) と素朴な細分化 (ENOMOTO_T_FOLD_NAIVE) の比べ。python summ_pt.py <pt_count.jsonl>
細分化の時間は LP_FOLD の行の "in X s" (畳む/畳まないを決めるまでの時間、問題の読み込みは含まない)。"""
import json, re, sys
from collections import defaultdict
d = defaultdict(dict)
for l in open(sys.argv[1]):
    r = json.loads(l); d[r["name"]][r["cfg"]] = r
def info(r):
    if not r or not r.get("fold"): return None
    l = r["fold"][0]
    t = float(re.search(r"in ([0-9.]+)s", l).group(1))
    m = re.search(r"n (\d+) -> (\d+), m (\d+) -> (\d+)", l)
    return t, (tuple(map(int, m.groups())) if m else None)
for fast, naive, name in (("rcnt", "rcntN", "反転あり"), ("cnt", "cntN", "反転なし")):
    rows = []
    for p, v in d.items():
        a, b = info(v.get(fast)), info(v.get(naive))
        if a and b: rows.append((p, a, b))
    ta = sum(r[1][0] for r in rows); tb = sum(r[2][0] for r in rows)
    diff = [(p, a[1], b[1]) for p, a, b in rows if a[1] != b[1]]
    print(f"{name}: {len(rows)} 問、細分化の時間の合計 速い {ta:.1f} 秒 / 素朴 {tb:.1f} 秒、畳んだ数 {sum(1 for r in rows if r[1][1])} / {sum(1 for r in rows if r[2][1])}、大きさの違う問題 {len(diff)}")
    for x in diff: print("   違い", x)
    big = sorted(rows, key=lambda r: -max(r[1][0], r[2][0]))[:12]
    for p, a, b in big: print(f"   {p:32s} 速い {a[0]:7.3f}  素朴 {b[0]:7.3f}  {a[1]}")
