"""RCSTRONG4 の集計: python summ.py [2] > summ[2].txt
(T) 狙いの問題の表 (形ごとに種 1〜4 の秒、* は時間切れ) と、問題ごとの形の比 (k に対する、shift 10 の幾何平均)。
(P) 58 問 × 種 1〜3 の対の比 (ik/k・gk/k、shift 1、時間切れは 60 秒、問題単位のブートストラップ)、解けた数、揃って遅い/速い問題。"""
import json, glob, math, random, collections, sys
SUF = sys.argv[1] if len(sys.argv) > 1 else ""  # "2": 直した gk の 2 回目 (res_T2・res_P2)
from pathlib import Path
D = Path(__file__).resolve().parent
FORMS = ["k", "ik", "gk"]
def load(s):
    R = {}
    for f in sorted(glob.glob(str(D / f"res_{s}_s*.jsonl"))):
        for l in open(f):
            r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
    return R
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
T = load("T" + SUF)
print(f"(T) 狙いの問題、600 秒 ({len(T)} 行)\n")
print("| 問題 | " + " | ".join(FORMS) + " | ik/k | gk/k |\n|---|---|---|---|---|---|")
for p in sorted({p for p, c, s in T}):
    cells, logs = [], {c: [] for c in FORMS}
    for c in FORMS:
        v = []
        for s in (1, 2, 3, 4):
            r = T.get((p, c, s))
            v.append("-" if r is None else f"{t(r):.0f}{'' if solved(r) else '*'}")
            if r is not None and (p, "k", s) in T:
                logs[c].append(math.log((t(r) + 10) / (t(T[(p, "k", s)]) + 10)))
        cells.append(" ".join(v))
    rat = [f"{math.exp(sum(logs[c]) / len(logs[c])):.2f}" if logs[c] else "-" for c in ("ik", "gk")]
    print(f"| {p} | " + " | ".join(cells) + " | " + " | ".join(rat) + " |")
P = load("P" + SUF)
print(f"\n(P) 58 問 × 種 1・2、60 秒 ({len(P)} 行)")
probs = sorted({p for p, c, s in P}); seeds = sorted({s for p, c, s in P})
for c in [c for c in FORMS if any(cc == c for (_, cc, _) in P)]:
    print(f"  {c}: 解けた {sum(solved(r) for (p, cc, s), r in P.items() if cc == c)}、最大 RSS {max([(r.get('max_rss_mb') or 0) for (p, cc, s), r in P.items() if cc == c] or [0])} MB")
bad = []
for (p, c, s), r in P.items():
    k = P.get((p, "k", s))
    if k and c != "k" and r["result"].get("status") == "optimal" and k["result"].get("status") == "optimal":
        a, b = k["result"]["objective"], r["result"]["objective"]
        if abs(a - b) > 1e-6 * max(1, abs(a)): bad.append((p, c, s, a, b))
print("  食い違い", len(bad), bad)
for c in [c for c in ("ik", "gk") if any(cc == c for (_, cc, _) in P)]:
    logr = collections.defaultdict(list)
    for p in probs:
        for s in seeds:
            a, b = P.get((p, "k", s)), P.get((p, c, s))
            if a and b and (solved(a) or solved(b)):
                logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]
        return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"  {c}/k: {sgm(ps) - 1:+.1%} [{bs[100] - 1:+.1%}, {bs[3899] - 1:+.1%}] (どちらかが解けた {len(ps)} 問)")
    for p in ps:
        v = logr[p]
        if len(v) == len(seeds) and all(x > math.log(1.25) for x in v): print("    遅", p, [round(math.exp(x), 2) for x in v])
        if len(v) == len(seeds) and all(x < math.log(0.8) for x in v): print("    速", p, [round(math.exp(x), 2) for x in v])
