"""abbench.py の結果 JSON を Markdown の表にする。

使い方: python analysis/mcf_cuts_20261028/summarize.py 結果.json miplib2017.solu [時間上限] [設定 A] [設定 B]
- 根の下界: 2 回の平均。根のギャップ = |ref - 根の下界| / max(|ref|, |根の下界|, 1) (scripts/mip_bench/bench.py の root_gap と同じ)
- 閉じた割合: (B の根の下界 - A の根の下界) / (ref - A の根の下界) (最小化。最大化の問題は符号を合わせる)
- 時間: 解けなければ上限。ずらし幾何平均 (ずらし 10 秒)
- 最後のギャップ: |ref - 最後の下界| / max(|ref|, 1) の平均 (解けた回は 0)
"""
import json, math, sys
from collections import defaultdict

res = json.load(open(sys.argv[1]))
ref = {}
for l in open(sys.argv[2]):
    p = l.split()
    if len(p) >= 3 and p[0] in ("=opt=", "=best="):
        ref[p[1]] = float(p[2])
tl = float(sys.argv[3]) if len(sys.argv) > 3 else 120.0
A = sys.argv[4] if len(sys.argv) > 4 else "def"
B = sys.argv[5] if len(sys.argv) > 5 else "mcf"
by = defaultdict(lambda: defaultdict(list))
for k, o in res.items():
    p, c, r = k.split("|")
    by[p][c].append(o)


def mean(v):
    v = [x for x in v if x is not None]
    return sum(v) / len(v) if v else None


def fmt(v, d=1):
    return "-" if v is None else f"{v:.{d}f}"


print(f"| 問題 | ref | 根の下界 {A} | 根の下界 {B} | 根のギャップ {A} % | 根のギャップ {B} % | 閉じた割合 % | 解けた {A} | 解けた {B} | 時間 {A} | 時間 {B} | ノード {A} | ノード {B} | 最後のギャップ {A} % | 最後のギャップ {B} % |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
sg = {A: [], B: []}
tot = {A: [0, 0], B: [0, 0]}
for p in sorted(by):
    rv = ref.get(p)
    row = [p, fmt(rv, 2) if rv is not None else "-"]
    rb, rg, sol, tm, nd, fg = {}, {}, {}, {}, {}, {}
    for c in (A, B):
        os_ = by[p].get(c, [])
        rb[c] = mean([o.get("root_bound") for o in os_])
        rg[c] = mean([min(1.0, abs(rv - o["root_bound"]) / max(abs(rv), abs(o["root_bound"]), 1.0)) * 100 if rv is not None and o.get("root_bound") is not None else None for o in os_])
        sol[c] = sum(o.get("status") == "optimal" for o in os_)
        ts = [o["time"] if o.get("status") == "optimal" and o.get("time") is not None else tl for o in os_]
        tm[c] = mean(ts)
        sg[c] += ts
        tot[c][0] += sol[c]
        tot[c][1] += len(os_)
        nd[c] = mean([o.get("nodes") for o in os_])
        fg[c] = mean([0.0 if o.get("status") == "optimal" else (min(1.0, abs(rv - o["best_bound"]) / max(abs(rv), 1.0)) * 100 if rv is not None and o.get("best_bound") is not None and math.isfinite(o["best_bound"]) else 100.0) for o in os_])
    closed = None
    if rv is not None and rb[A] is not None and rb[B] is not None and abs(rv - rb[A]) > 1e-9 * max(1.0, abs(rv)):
        closed = (rb[B] - rb[A]) / (rv - rb[A]) * 100
    row += [fmt(rb[A], 2), fmt(rb[B], 2), fmt(rg[A], 3), fmt(rg[B], 3), fmt(closed, 1), f"{sol[A]}/{len(by[p].get(A, []))}", f"{sol[B]}/{len(by[p].get(B, []))}", fmt(tm[A]), fmt(tm[B]), fmt(nd[A], 0), fmt(nd[B], 0), fmt(fg[A], 3), fmt(fg[B], 3)]
    print("| " + " | ".join(row) + " |")
for c in (A, B):
    t = sg[c]
    if t:
        s = math.exp(sum(math.log(x + 10) for x in t) / len(t)) - 10
        print(f"\n{c}: 解けた {tot[c][0]}/{tot[c][1]} 回、時間のずらし幾何平均 {s:.2f} 秒")
