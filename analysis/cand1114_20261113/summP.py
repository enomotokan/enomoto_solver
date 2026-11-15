"""cand1114 の 58 問 × 種 1〜4 (60 秒) の対の比較 (../cand1113_20261112/summP.py の流用)。python summP.py [res のあるディレクトリ] > summP.txt
SGM は shift 1 秒、時間切れは制限時間で数える。CI は問題単位のブートストラップ。最後に、組み直しで外した項目の問題の比を cand1113 の P と並べる。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent
C13 = Path(__file__).resolve().parents[1] / "cand1113_20261112"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_P_s*.jsonl"))) for l in open(f)]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def t(r):
    res = r["result"]
    if res.get("status") == "optimal" or res.get("status") == "infeasible":
        return min(res["time"], r["tl"])
    return r["tl"]
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
probs = sorted({p for p, c, s in R})
seeds = sorted({s for p, c, s in R})
logr = collections.defaultdict(list)
ns = {"def": 0, "cand1114": 0}; crash = 0; bad = []
for p in probs:
    for s in seeds:
        a, b = R.get((p, "def", s)), R.get((p, "cand1114", s))
        if not a or not b: continue
        for x in (a, b):
            if "error" in x["result"] or x["result"].get("status") in (None, "crash", "error"): crash += 1
        ns["def"] += solved(a); ns["cand1114"] += solved(b)
        if solved(a) and solved(b) and a["result"]["status"] == "optimal" and b["result"]["status"] == "optimal":
            oa, ob = a["result"]["objective"], b["result"]["objective"]
            if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
        if a["result"].get("status") != b["result"].get("status") and {a["result"].get("status"), b["result"].get("status")} == {"optimal", "infeasible"}:
            bad.append((p, s, a["result"]["status"], b["result"]["status"]))
        if solved(a) or solved(b):
            logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
def sgm(ps): 
    v = [x for p in ps for x in logr[p]]
    return math.exp(sum(v) / len(v)) if v else 1.0
ps = list(logr)
est = sgm(ps); random.seed(0)
bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"行数 {len(rows)}、crash {crash}、食い違い {len(bad)} {bad}")
print(f"解けた数 (58 問 × 種 1〜4 + physiciansched6-2 の種 5〜12): def {ns['def']} / cand1114 {ns['cand1114']}")
print(f"対の比 (どちらかが解けた {len(ps)} 問): {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
print("種をまたいで揃って遅い/速い問題 (その問題の全種で比 >1.25 / <0.8):")
for p in ps:
    v = logr[p]
    if len(v) >= 4 and all(x > math.log(1.25) for x in v): print("  遅", p, [round(math.exp(x), 2) for x in v])
    if len(v) >= 4 and all(x < math.log(0.8) for x in v): print("  速", p, [round(math.exp(x), 2) for x in v])
for c in ("def", "cand1114"):
    print(c, "最大 RSS", max(r["max_rss_mb"] for r in rows if r["cfg"] == c), "MB")

# 組み直しで外した項目の問題 (cand1113 で揃って遅い/速かった問題): 比 = 束 / def の時間 (shift 1)、種ごと。cand1113 の P と並べる
r13 = {(r["problem"], r["cfg"], r["seed"]): r for f in sorted(glob.glob(str(C13 / "res_P_s*.jsonl"))) for l in open(f) for r in [json.loads(l)]}
def rat(RR, p, c, s):
    a, b = RR.get((p, "def", s)), RR.get((p, c, s))
    return f"{(t(b) + 1) / (t(a) + 1):.2f}" if a and b else "-"
print("組み直しの確かめ (束 / def、種 1〜4。cand1113 → cand1114):")
for p, why in (("neos-960392", "SUBFIX=0 を外した"), ("binkar10_1", "DUALEQ を外した"), ("roll3000", "CLQPRB を外した"), ("pk1", "GAP1=u (残した)"),
               ("neos8", "CLQPRB の効き (外した)"), ("h80x6320d", "ewk の効き"), ("10teams", "揺れ")):
    print(f"  {p} ({why}): " + " ".join(rat(r13, p, "cand1113", s) for s in (1, 2, 3, 4)) + " → " + " ".join(rat(R, p, "cand1114", s) for s in (1, 2, 3, 4)))
