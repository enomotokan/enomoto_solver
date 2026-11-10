"""LP の時間の集計 (analysis/drcr_20261110.md)。python summ_lp.py <jsonl> [上限秒]
時間は状態が optimal ならその時間、ほかは上限。SGM のずらしは 1 秒。目的値は def との相対差 1e-6 で比べる。"""
import json, math, re, sys
from collections import defaultdict
TL = float(sys.argv[2]) if len(sys.argv) > 2 else 120.0
d = defaultdict(dict)
for l in open(sys.argv[1]):
    r = json.loads(l); d[r["name"]][r["cfg"]] = r
def t(r): return r["time"] if r.get("status") == "optimal" else TL
def size(r):
    m = r.get("fold") and re.search(r"n (\d+) -> (\d+), m (\d+) -> (\d+)", r["fold"][0])
    return tuple(map(int, m.groups())) if m else None
def sgm(xs, s=1.0): return math.exp(sum(math.log(x + s) for x in xs) / len(xs)) - s
rows = []; bad = []
for n, v in sorted(d.items()):
    if not all(c in v for c in ("def", "fold", "rfold")): continue
    o = v["def"].get("obj")
    for c in ("fold", "rfold"):
        oc = v[c].get("obj")
        if v["def"].get("status") == "optimal" and v[c].get("status") == "optimal" and abs(o - oc) > 1e-6 * (1 + abs(o)):
            bad.append((n, c, o, oc))
        if (v["def"].get("status") == "optimal") != (v[c].get("status") == "optimal"):
            bad.append((n, c, v["def"].get("status"), v[c].get("status")))
    rows.append((n, t(v["def"]), t(v["fold"]), t(v["rfold"]), size(v["fold"]), size(v["rfold"]), [l for c in ("fold", "rfold") for l in v[c]["fold"] if "safety" in l or "ended" in l]))
print(f"問題 {len(rows)}、状態か目的値の食い違い {len(bad)}")
for b in bad: print("  ", b)
def line(name, sel):
    if not sel: return
    a, b, c = [x[1] for x in sel], [x[2] for x in sel], [x[3] for x in sel]
    print(f"{name}: {len(sel)} 問  SGM def {sgm(a):.3f}  DRCR {sgm(b):.3f} ({sgm(b)/sgm(a):.2f})  R-DRCR {sgm(c):.3f} ({sgm(c)/sgm(a):.2f})  合計 {sum(a):.1f} / {sum(b):.1f} / {sum(c):.1f}")
line("全部", rows)
line("def 10 秒超", [r for r in rows if r[1] > 10])
line("def 1 秒超", [r for r in rows if r[1] > 1])
line("反転で DRCR より縮む", [r for r in rows if r[5] and (not r[4] or r[5][1] + r[5][3] < r[4][1] + r[4][3])])
print()
print("| 問題 | 列 n | 行 m | DRCR 後 (列, 行) | R-DRCR 後 (列, 行) | def 秒 | DRCR 秒 | R-DRCR 秒 |")
print("|---|---|---|---|---|---|---|---|")
for n, a, b, c, s1, s2, sn in rows:
    s = s1 or s2
    f = lambda s: f"{s[1]}, {s[3]}" if s else "-"
    print(f"| {n} | {s[0] if s else ''} | {s[2] if s else ''} | {f(s1)} | {f(s2)} | {a:.2f} | {b:.2f} | {c:.2f} |" + (" 安全網" if sn else ""))
