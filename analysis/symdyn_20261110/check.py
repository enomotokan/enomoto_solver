"""誤答の検査: 全ての回で、暫定解と下界の矛盾 (最小化の形で 下界 > 暫定解、または ある回の下界 > 別の回の暫定解、
optimal の値の不一致) がないか。python check.py res*.jsonl"""
import json, math, sys
from collections import defaultdict
rows = [json.loads(l) for f in sys.argv[1:] for l in open(f) if l.strip()]
by = defaultdict(list)
for r in rows:
    by[r["problem"]].append(r)
bad = 0
for p, rs in sorted(by.items()):
    vals = [(r["cfg"], r.get("rep", r.get("seed")), r["result"]) for r in rs if r["result"].get("objective") is not None]
    if not vals:
        continue
    # 向き: 下界 <= 暫定解 の回が多いほう
    mn = sum(1 for _, _, x in vals if x.get("bound") is not None and x["bound"] <= x["objective"] + 1e-6 * (1 + abs(x["objective"])))
    sgn = 1 if mn >= len(vals) / 2 else -1
    tol = lambda a: 1e-6 * (1 + abs(a))
    best_inc = min(sgn * x["objective"] for _, _, x in vals)
    opts = [sgn * x["objective"] for _, _, x in vals if x.get("status") == "optimal"]
    for c, k, x in vals:
        b = x.get("bound")
        if b is None or not math.isfinite(b):
            continue
        if sgn * b > best_inc + tol(best_inc):
            bad += 1
            print(f"矛盾 {p} {c} 回 {k}: 下界 {b} > 最良の暫定解 {sgn * best_inc}")
    if opts and max(opts) - min(opts) > tol(min(opts)):
        bad += 1
        print(f"optimal の不一致 {p}: {sorted(set(round(sgn * o, 6) for o in opts))}")
print(f"問題 {len(by)}、回 {len(rows)}、矛盾 {bad}")
