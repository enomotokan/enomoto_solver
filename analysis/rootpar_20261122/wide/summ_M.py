"""M (Mittelmann、4 スレッド・600 秒、種 1) の def/rp 比較のまとめ: 状態の内訳、最適値の食い違い、片方だけ解けた問題、
rp が速い/遅い問題 (1.25 倍)、時間切れの暫定解・下界の良し悪しと比の分布。使い方: python summ_M.py res_M_s*.jsonl"""
import json, sys, math
from collections import defaultdict, Counter
by = defaultdict(dict)
for f in sys.argv[1:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l); by[r["problem"]][r["cfg"]] = r["result"]
st = Counter(); mis = []; one = []; fast = []; slow = []
tl_cmp = Counter(); tl_rows = []; bd = Counter()
def rel(a, b):
    return (a - b) / max(1.0, abs(b))
for p, d in sorted(by.items()):
    if "def" not in d or "rp" not in d: continue
    a, b = d["def"], d["rp"]
    for c, x in (("def", a), ("rp", b)): st[(c, x.get("status"))] += 1
    sa, sb = a.get("status"), b.get("status")
    if sa == "optimal" and sb == "optimal":
        oa, ob = a["objective"], b["objective"]
        if abs(oa - ob) > 1e-6 * max(1, abs(oa)): mis.append((p, oa, ob))
        ta, tb = a["time"], b["time"]
        r = (tb + 10) / (ta + 10)
        if tb > 1.25 * ta or r > 1.25: slow.append((p, round(ta, 1), round(tb, 1), round(r, 2)))
        if ta > 1.25 * tb and r < 0.8: fast.append((p, round(ta, 1), round(tb, 1), round(r, 2)))
    elif (sa == "optimal") != (sb == "optimal"):
        one.append((p, sa, sb, a.get("time"), b.get("time")))
    if sa == "time_limit" and sb == "time_limit":
        oa, ob = a.get("objective"), b.get("objective")
        if oa is None and ob is None: tl_cmp["両方解なし"] += 1; continue
        if oa is None: tl_cmp["rp 良 (def 解なし)"] += 1; tl_rows.append((p, None, ob, None)); continue
        if ob is None: tl_cmp["rp 悪 (rp 解なし)"] += 1; tl_rows.append((p, oa, None, None)); continue
        x = rel(ob, oa)  # 最小化: 負なら rp が良い
        k = "同じ" if abs(x) < 1e-6 else ("rp 良" if x < 0 else "rp 悪")
        tl_cmp[k] += 1; tl_rows.append((p, oa, ob, x))
        ba, bb = a.get("bound"), b.get("bound")
        if ba is not None and bb is not None:
            y = rel(bb, ba); bd["同じ" if abs(y) < 1e-6 else ("rp 上" if y > 0 else "rp 下")] += 1
print("問題数", len(by)); print("状態:", dict(st))
print("解けた def", sum(v for (c, s), v in st.items() if c == "def" and s == "optimal"), "/ rp", sum(v for (c, s), v in st.items() if c == "rp" and s == "optimal"))
print("最適値の食い違い:", mis or "なし"); print("片方だけ解けた (問題, def, rp, 時間):", one or "なし")
print("rp が 1.25 倍超遅い (def 秒, rp 秒, ずらした比):", slow); print("rp が 1.25 倍超速い:", fast)
print("時間切れ (両方) の暫定解:", dict(tl_cmp)); print("時間切れの下界:", dict(bd))
xs = sorted(x for *_, x in tl_rows if x is not None)
if xs:
    q = lambda f: xs[min(len(xs) - 1, int(f * len(xs)))]
    print(f"暫定解の相対差 (rp-def)/max(1,|def|) の分布 n={len(xs)}: 最小 {xs[0]:.4f} 25% {q(.25):.4f} 中央 {q(.5):.4f} 75% {q(.75):.4f} 最大 {xs[-1]:.4f}")
    print("  |差|>1% の問題:", [(p, round(x, 4)) for p, _, _, x in tl_rows if x is not None and abs(x) > 0.01])
