"""combo3 s2 (MIPLIB 240 問、種 1・600 秒、ビルド 4d84a33cd ほか) の既定 1T・4T から、4T / 1T の速さの比の分布と問題の型を出す。
python speedup.py > speedup.txt"""
import json, glob, math
from pathlib import Path
S2 = Path(__file__).resolve().parents[1] / "combo3_20261126/s2"
rows = {}
for f in sorted(glob.glob(str(S2 / "res_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] in ("def_t1", "def_t4"): rows[(r["problem"], r["cfg"])] = r
P = sorted({p for p, c in rows})
TL = 600.0
def t(r): return r["result"]["time"] if r["result"]["status"] == "optimal" else TL
def kind(a, b):
    """型: 1T のノード数とノードの速さ、最初の解の時刻から (目安の分け方。閾値は表を読みやすくするためだけ)"""
    ra, rb = a["result"], b["result"]
    n1 = ra["nodes"] or 0; t1 = t(a)
    ks = []
    if n1 <= 10: ks.append("根で閉じる")
    elif n1 / max(t1, 1e-3) < 20: ks.append("LP が重い")
    else: ks.append("木が大きい" if n1 > 10000 else "木が中くらい")
    f1 = ra.get("first_solution_time")
    if n1 > 10 and (f1 is None or f1 > 0.3 * t1): ks.append("解探しが難しい")
    return "・".join(ks)
out = []
for p in P:
    a, b = rows.get((p, "def_t1")), rows.get((p, "def_t4"))
    if not a or not b: continue
    s1, s4 = a["result"]["status"] == "optimal", b["result"]["status"] == "optimal"
    out.append((p, s1, s4, (t(a) + 1) / (t(b) + 1), a, b))
cnt = lambda f: sum(1 for x in out if f(x))
print(f"{len(out)} 問: 両方で解けた {cnt(lambda x: x[1] and x[2])}、4T だけ {cnt(lambda x: x[2] and not x[1])}、1T だけ {cnt(lambda x: x[1] and not x[2])}、どちらも解けない {cnt(lambda x: not x[1] and not x[2])}")
both = sorted((x for x in out if x[1] and x[2]), key=lambda x: x[3])
r = [x[3] for x in both]
print("両方で解けた問題の 1T 時間 / 4T 時間 (shift 1 秒): SGM %.2f、分位 (最小・10%%・25%%・中央・75%%・90%%・最大) %s" % (
    math.exp(sum(map(math.log, r)) / len(r)), [round(r[int(q * (len(r) - 1))], 2) for q in (0, .1, .25, .5, .75, .9, 1)]))
bins = [(0, .8), (.8, 1.25), (1.25, 2), (2, 3), (3, 100)]
print("比の度数: " + "、".join(f"[{lo},{hi}) {sum(lo <= x < hi for x in r)}" for lo, hi in bins))
# 1T 時間で分けた比 (重い問題ほど伸びるか)
for lo, hi in ((0, 10), (10, 60), (60, 600)):
    rr = [x[3] for x in both if lo <= t(x[4]) < hi]
    if rr: print(f"  1T の時間 [{lo},{hi}) 秒: {len(rr)} 問、SGM {math.exp(sum(map(math.log, rr)) / len(rr)):.2f}")
print("\n型ごと (両方で解けた問題): 問題数・SGM・4T が遅い (比 < 1) 数")
ks = {}
for x in both: ks.setdefault(kind(x[4], x[5]), []).append(x[3])
for k, v in sorted(ks.items(), key=lambda kv: -len(kv[1])):
    print(f"  {k}: {len(v)} 問、SGM {math.exp(sum(map(math.log, v)) / len(v)):.2f}、比 < 1 は {sum(y < 1 for y in v)}")
print("\n4T だけ解けた問題 (1T は 600 秒で時間切れ): 4T の時間・ノード、型")
for x in out:
    if x[2] and not x[1]: print(f"  {x[0]}: 4T {t(x[5]):.0f} 秒 {x[5]['result']['nodes']} ノード / 1T {x[4]['result']['nodes']} ノード  {kind(x[4], x[5])}")
print("\n両方で解けた問題の一覧 (比の小さい順 = 4T が伸びない順)。時間 秒・ノード・最初の解の時刻")
for p, s1, s4, x, a, b in both:
    ra, rb = a["result"], b["result"]
    n1, n4 = ra["nodes"] or 0, rb["nodes"] or 0
    print(f"  {p:28} 比 {x:5.2f}  1T {t(a):6.1f} 秒 {n1:8} ノード 最初の解 {ra.get('first_solution_time') or float('nan'):6.1f} | 4T {t(b):6.1f} 秒 {n4:8} ノード 最初の解 {rb.get('first_solution_time') or float('nan'):6.1f}  ノード比 4T/1T {(n4 + 1) / (n1 + 1):5.2f}  {kind(a, b)}")
# どちらも解けない問題: 下界の伸び
print("\nどちらも解けない問題: 4T の暫定解が良い/悪い/同じ、下界が良い/悪い (最小化に直して)")
sol = json.loads((S2 / "solu.json").read_text()) if (S2 / "solu.json").exists() else {}
def gap(r):
    o, b = r["result"].get("objective"), r["result"].get("bound")
    if o is None or b is None: return 1.0
    return min(1.0, abs(o - b) / max(abs(o), abs(b), 1e-9))
nn = [x for x in out if not x[1] and not x[2]]
g1 = [gap(x[4]) for x in nn]; g4 = [gap(x[5]) for x in nn]
print(f"  {len(nn)} 問: 600 秒の時点の相対ギャップ 4T < 1T {sum(b < a - 1e-9 for a, b in zip(g1, g4))}、4T > 1T {sum(b > a + 1e-9 for a, b in zip(g1, g4))}、同じ {sum(abs(a - b) <= 1e-9 for a, b in zip(g1, g4))}")
print(f"  暫定解なし: 1T {sum(x[4]['result'].get('objective') is None for x in nn)}、4T {sum(x[5]['result'].get('objective') is None for x in nn)}")
worse = sorted(((gap(x[5]) - gap(x[4]), x[0], gap(x[4]), gap(x[5])) for x in nn), reverse=True)[:12]
print("  4T のギャップが大きく悪い問題: " + "、".join(f"{p} {a:.3f}→{b:.3f}" for _, p, a, b in worse if _ > 0.005))
