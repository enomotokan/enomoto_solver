"""OBJTARGET: 対の追加の集計 (summ.py の後): python summ2.py SET [試作] → 全種で 1.25 倍超の遅速 (解けた問題の時間)、
時間切れの組の最後の暫定解の良し悪し (相対 1e-6 で比べる) の数、主の積分が全種で同じ向きに 10% 超変わる問題 (種で向きが割れる問題は別に)。"""
import json, glob, math, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.argv += ["l"] if len(sys.argv) < 3 else []
SET, ALT = sys.argv[1], sys.argv[2]
exec(open(D / "summ.py").read().split("crash =")[0].replace("SET = sys.argv[1]", f"SET = {SET!r}").replace('ALT = sys.argv[2] if len(sys.argv) > 2 else "l"', f"ALT = {ALT!r}"))
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
full = [(p, s) for p in probs for s in seeds if (p, "def", s) in R and (p, ALT, s) in R]
by = collections.defaultdict(list)
for p, s in full: by[p].append(s)
slow, fast = [], []
for p, ss in by.items():
    rs = [(t(R[(p, ALT, s)]) + 1) / (t(R[(p, "def", s)]) + 1) for s in ss if solved(R[(p, "def", s)]) or solved(R[(p, ALT, s)])]
    if len(rs) == len(ss) and len(ss) >= 2:
        if all(x > 1.25 for x in rs): slow.append((p, [round(x, 2) for x in rs]))
        if all(x < 0.8 for x in rs): fast.append((p, [round(x, 2) for x in rs]))
print(f"{SET} {ALT}/def 全種で 1.25 倍超遅い: {slow}")
print(f"{SET} 全種で 1.25 倍超速い: {fast}")
better = worse = same = 0
for p, s in full:
    a, b = R[(p, "def", s)], R[(p, ALT, s)]
    if st(a) == "time_limit" and st(b) == "time_limit":
        za, zb = a["result"].get("objective"), b["result"].get("objective")
        if za is None and zb is None: same += 1; continue
        if za is None: better += 1; continue
        if zb is None: worse += 1; continue
        tol = 1e-6 * max(1, abs(za))
        if zb < za - tol: better += 1
        elif zb > za + tol: worse += 1
        else: same += 1
print(f"{SET} 両方時間切れの組: {ALT} が良い {better}・悪い {worse}・同じ {same}")
cons_b, cons_w, split = [], [], []
for p, ss in by.items():
    v = []
    for s in ss:
        ia, ib = pint(R[(p, "def", s)]), pint(R[(p, ALT, s)])
        if ia is None: break
        v.append((ib + 1) / (ia + 1))
    if len(v) != len(ss) or len(v) < 2: continue
    if all(x < 0.9 for x in v): cons_b.append((p, [round(x, 2) for x in v]))
    elif all(x > 1.1 for x in v): cons_w.append((p, [round(x, 2) for x in v]))
    elif min(v) < 0.9 and max(v) > 1.1: split.append((p, [round(x, 2) for x in v]))
print(f"{SET} 主の積分が全種で 10% 超良い ({len(cons_b)}): {sorted(cons_b, key=lambda x: sum(x[1]))}")
print(f"{SET} 全種で 10% 超悪い ({len(cons_w)}): {sorted(cons_w, key=lambda x: -sum(x[1]))}")
print(f"{SET} 種で向きが割れる ({len(split)}): {[p for p, _ in split]}")
