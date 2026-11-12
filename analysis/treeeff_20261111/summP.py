"""TREEEFF の 58 問 × 種 1〜3 (60 秒) の対の比較: [SET=P2] python summP.py [候補 ...] (既定は res_P_s*.jsonl にある def 以外の設定全部)。
SGM は shift 1 秒、時間切れは制限時間で数える。区間は問題単位のブートストラップ (95%)。cand1111 の summP.py と同じ数え方。"""
import json, glob, math, random, collections, sys
import os
SET = os.environ.get("SET", "P")
rows = [json.loads(l) for f in sorted(glob.glob(f"res_{SET}_s*.jsonl")) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
cands = sys.argv[1:] or sorted({c for p, c, s in R} - {"def"})
crash = sum(1 for r in rows if r["result"].get("status") in (None, "crash", "error", "fetch_error"))
print(f"行数 {len(rows)}、crash {crash}")
for c in cands:
    logr = collections.defaultdict(list); ns = [0, 0]; bad = []; n = 0
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "def", s)), R.get((p, c, s))
            if not a or not b: continue
            n += 1; ns[0] += solved(a); ns[1] += solved(b)
            sa, sb = a["result"].get("status"), b["result"].get("status")
            if sa == sb == "optimal":
                oa, ob = a["result"]["objective"], b["result"]["objective"]
                if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
            if {sa, sb} == {"optimal", "infeasible"}: bad.append((p, s, sa, sb))
            if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    def sgm(ps):
        v = [x for p in ps for x in logr[p]]
        return math.exp(sum(v) / len(v)) if v else 1.0
    ps = list(logr); est = sgm(ps); random.seed(0)
    bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"\n== {c}: 対 {n} 組、食い違い {len(bad)} {bad}")
    print(f"解けた数: def {ns[0]} / {c} {ns[1]}")
    print(f"対の比 (どちらかが解けた {len(ps)} 問): {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
    for p in ps:
        v = logr[p]
        if len(v) == len(seeds) and all(x > math.log(1.25) for x in v): print("  全種で遅い", p, [round(math.exp(x), 2) for x in v])
        if len(v) == len(seeds) and all(x < math.log(0.8) for x in v): print("  全種で速い", p, [round(math.exp(x), 2) for x in v])
    print("  最大 RSS", max(r["max_rss_mb"] for r in rows if r["cfg"] == c), "MB (def", max(r["max_rss_mb"] for r in rows if r["cfg"] == "def"), "MB)")
