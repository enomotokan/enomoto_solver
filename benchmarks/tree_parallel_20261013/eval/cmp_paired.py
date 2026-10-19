"""paired.py の結果を比べる: cmp_paired.py OUT.jsonl [BASE]
完成したグループ (全設定がそろい、同じ boot_id) だけを使う。
- SGM: 未解決は時間上限、shift 10 秒。回ごとに問題の時間を幾何平均してから。
- 対の比: 問題ごとの (時間+10)/(基準の時間+10) の幾何平均 (マシンの違いに強い)。
- ギャップ: 全設定・全回で時間切れの問題の、時間切れのときの相対ギャップ (%) の shift 1 の幾何平均。"""
import json, math, sys
from collections import defaultdict
recs = [json.loads(l) for l in open(sys.argv[1])]
cfgs = list(dict.fromkeys(r["cfg"] for r in recs))
base = sys.argv[2] if len(sys.argv) > 2 else cfgs[0]
G = defaultdict(dict)
for r in recs:
    G[(r["round"], r["name"])][r["cfg"]] = r
G = {k: v for k, v in G.items() if set(v) >= set(cfgs) and len({r["boot"] for r in v.values()}) == 1}
ok = lambda r: r.get("status") in ("optimal", "Optimal")
tl = max(r.get("time") or 0 for r in recs)
tl = 60.0 if tl < 90 else round(tl)
tm = lambda r: r["time"] if ok(r) and r.get("time") is not None else tl
def gap(r):
    o, b = r.get("objective"), r.get("bound")
    if o is None or b is None or not math.isfinite(b): return 100.0
    return min(100.0, 100 * abs(o - b) / max(abs(o), 1e-9))
gm = lambda v, s: math.exp(sum(math.log(x + s) for x in v) / len(v)) - s
names = sorted({k[1] for k in G})
per_prob = {n: [k for k in G if k[1] == n] for n in names}
names = [n for n in names if per_prob[n]]
tout = [n for n in names if all(not ok(G[k][c]) for k in per_prob[n] for c in cfgs)]
boots = {r["boot"] for g in G.values() for r in g.values()}
print(f"完成したグループ {len(G)} (問題 {len(names)}、マシン {len(boots)} 台)")
bs = None
for c in cfgs:
    sgm = gm([gm([tm(G[k][c]) for k in per_prob[n]], 10) for n in names], 10)
    ratio = math.exp(sum(math.log((tm(G[k][c]) + 10) / (tm(G[k][base]) + 10)) for k in G) / len(G))
    gp = gm([gm([gap(G[k][c]) for k in per_prob[n]], 1) for n in tout], 1) if tout else float("nan")
    solved = sum(ok(G[k][c]) for k in G) / max(1, len(G) / len(names))
    bs = bs or (sgm, gp)
    print(f"{c:12s} SGM {sgm:6.2f} ({100*(sgm/bs[0]-1):+5.1f}%) 対の比 {100*(ratio-1):+5.1f}% 解けた {solved:4.1f} | 時間切れ{len(tout)}問のギャップ {gp:6.2f}% ({100*(gp/bs[1]-1):+5.1f}%)")
