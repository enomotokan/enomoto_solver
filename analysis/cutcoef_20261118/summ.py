"""cutcoef 段 1 の集計: python summ.py [res_G_s*.jsonl ...]
- 設定ごと: 解けた数、SGM (シフト 1 秒、解けない回は 60 秒) の def 比。
- 伸びしろ: 問題ごとに種の平均で最良の組を選んだときの SGM (上限、選びと評価が同じ種なので甘い) と、
  種を 1 つ抜いて残りで選び抜いた種で評価したとき (抜き 1 の交差検証、正直な上限)。
- 安定さ: 問題ごとに、種どうしの設定の順位の Spearman 相関の平均と、種ごとの最良の組が一致する割合。"""
import json, glob, sys, math, itertools
from collections import defaultdict
fs = sys.argv[1:] or sorted(glob.glob("res_G_s*.jsonl"))
R = {}
crash = 0
for f in fs:
    for l in open(f):
        d = json.loads(l); r = d["result"]; s = str(r.get("status", "")).lower()
        if s in ("crash", "fetch_error"): crash += 1
        t = min(r.get("time") or 60, 60) if s == "optimal" else 60.0
        R[(d["problem"], d["cfg"], d["seed"])] = (t, s == "optimal")
P = sorted({k[0] for k in R}); C = sorted({k[1] for k in R}, key=lambda c: (c != "def", c)); S = sorted({k[2] for k in R})
SP = {p: [s for s in S if all((p, c, s) in R for c in C)] for p in P}
full = [p for p in P if len(SP[p]) >= 2]
print(f"行 {len(R)}、crash {crash}、2 種以上そろった問題 {len(full)}/{len(P)}、設定 {len(C)}、種 {S}")
def sgm(ts): return math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1
base = sgm([R[(p, "def", s)][0] for p in full for s in SP[p]])
print(f"\n設定ごと (SGM の def 比、解けた数 / {sum(len(SP[p]) for p in full)}、害: def 比の最大・1.25 倍超の回数・def が解けて自分が解けない回数。害は全部の (問題, 種) で数える):")
for c in C:
    ts = [R[(p, c, s)][0] for p in full for s in SP[p]]
    pr = [(p, s) for (p, cc, s) in R if cc == c and (p, "def", s) in R]
    rt = [((R[(p, c, s)][0] + 1) / (R[(p, "def", s)][0] + 1), p, s) for p, s in pr]
    mx = max(rt) if rt else (0, "", 0)
    lost = sum(1 for p, s in pr if R[(p, "def", s)][1] and not R[(p, c, s)][1])
    print(f"  {c:10s} {sgm(ts) / base:6.3f}  {sum(R[(p, c, s)][1] for p in full for s in SP[p]):3d}   最大 {mx[0]:5.2f} ({mx[1]} 種 {mx[2]})  1.25 倍超 {sum(1 for r in rt if r[0] > 1.25):2d}/{len(rt)}  解けなくなる {lost}")
m = {(p, c): sum(math.log(R[(p, c, s)][0] + 1) for s in SP[p]) / len(SP[p]) for p in full for c in C}
best = {p: min(C, key=lambda c: m[(p, c)]) for p in full}
print(f"\n上限 (種の平均で最良の組): {sgm([R[(p, best[p], s)][0] for p in full for s in SP[p]]) / base:.3f}")
cv = []
for p in full:
    for s in SP[p]:
        o = [x for x in SP[p] if x != s]
        b = min(C, key=lambda c: sum(math.log(R[(p, c, x)][0] + 1) for x in o))
        cv.append(R[(p, b, s)][0])
print(f"抜き 1 の交差検証: {sgm(cv) / base:.3f}")
def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r = [0] * len(v)
    for k, i in enumerate(o): r[i] = k
    return r
def spear(a, b):
    ra, rb = rank(a), rank(b); n = len(a)
    return 1 - 6 * sum((x - y) ** 2 for x, y in zip(ra, rb)) / (n * (n * n - 1))
print("\n問題ごと (def の秒の平均、最良の組と def 比、種ごとの最良、順位相関の平均、交差検証の def 比):")
for p in full:
    rhos = [spear([R[(p, c, a)][0] for c in C], [R[(p, c, b)][0] for c in C]) for a, b in itertools.combinations(SP[p], 2)]
    bs = [min(C, key=lambda c: R[(p, c, s)][0]) for s in SP[p]]
    d = sum(R[(p, "def", s)][0] for s in SP[p]) / len(SP[p])
    cvp = []
    for s in SP[p]:
        o = [x for x in SP[p] if x != s]
        b = min(C, key=lambda c: sum(math.log(R[(p, c, x)][0] + 1) for x in o))
        cvp.append((R[(p, b, s)][0] + 1) / (R[(p, "def", s)][0] + 1))
    print(f"  {p:18s} def {d:5.1f}  最良 {best[p]:10s} {math.exp(m[(p, best[p])] - m[(p, 'def')]):5.2f}  種ごと {','.join(bs):34s} ρ {sum(rhos) / len(rhos):5.2f}  cv {math.exp(sum(map(math.log, cvp)) / len(cvp)):5.2f}")
