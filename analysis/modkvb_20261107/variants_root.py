"""6 章: 58 問の根 (種 1・2) で、mk50 の形の違い (mk50 = pyw4、mk50z = 同じ形を pyw6 で数え上げつき、mk50cg = CG の式だけ、
mk50cgb = CG の式 + 従来、mk50skz = zerohalf と平行な候補を除く) を既定と比べ、2 回とも上 / 下の問題を数える。使い方: python variants_root.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "root58.jsonl"):
    r = json.loads(l); R.setdefault(r["problem"], {})[(r["cfg"], r.get("rep", 1))] = r["result"].get("bound")
cfgs = ["mk50", "mk50z", "mk50cg", "mk50cgb", "mk50skz", "nozh", "mk50nozh"]
res = {c: ([], []) for c in cfgs}
for p, d in R.items():
    try:
        a, b = d[("def", 1)], d[("def", 2)]
    except KeyError:
        continue
    if a is None or b is None: continue
    s = 1e-6 * max(1, abs(a))
    for c in cfgs:
        x, y = d.get((c, 1)), d.get((c, 2))
        if x is None or y is None: continue
        if min(x, y) > max(a, b) + s: res[c][0].append(p)
        elif max(x, y) < min(a, b) - s: res[c][1].append(p)
for c in cfgs:
    print(f"{c:9s} 2 回とも上 {len(res[c][0])}: {res[c][0]}\n{'':9s} 2 回とも下 {len(res[c][1])}: {res[c][1]}")
