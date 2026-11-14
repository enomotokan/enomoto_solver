"""timedep の 58 問 × 種 1・2 の、各設定と base の対の比較: python summ.py > summ.txt
SGM は shift 1 秒、時間切れは制限時間で数える。区間は問題単位のブートストラップ (95%)。食い違い = 両方が最適で目的値が違う、または最適と実行不能。
「揃って遅い」は全種で比 > 1.25 (速いは < 0.8)。最後に、各設定の同じ (問題) の種 1・2 の時間の比 (種の間の揺れ) の幾何平均を並べる。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def t(r):
    res = r["result"]
    return min(res["time"], r["tl"]) if res.get("status") in ("optimal", "infeasible") else r["tl"]
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
print(f"行数 {len(rows)}、問題 {len(probs)}、種 {seeds}")
crash = [(p, c, s) for (p, c, s), r in R.items() if "error" in r["result"] or r["result"].get("status") in (None, "crash", "error")]
print(f"crash {len(crash)} {crash[:10]}")
for c in CFGS:
    if c == "base": continue
    logr = collections.defaultdict(list); ns = [0, 0]; bad = []; npair = 0
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "base", s)), R.get((p, c, s))
            if not a or not b: continue
            npair += 1; ns[0] += solved(a); ns[1] += solved(b)
            sa, sb = a["result"].get("status"), b["result"].get("status")
            if sa == sb == "optimal":
                oa, ob = a["result"]["objective"], b["result"]["objective"]
                if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
            if {sa, sb} == {"optimal", "infeasible"}: bad.append((p, s, sa, sb))
            if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(pp):
        v = [x for q in pp for x in logr[q]]
        return math.exp(sum(v) / len(v)) if v else 1.0
    est = sgm(ps); random.seed(0)
    bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"\n## {c} ({CFGS[c].get('ENOMOTO_MIP_TIMEDEP')}) 対 {npair}: 解けた base {ns[0]} / {c} {ns[1]}、食い違い {len(bad)} {bad}")
    print(f"  SGM 比 (どちらかが解けた {len(ps)} 問): {est - 1:+.1%} [{bs[100] - 1:+.1%}, {bs[3899] - 1:+.1%}]")
    for p in ps:
        v = logr[p]
        if len(v) >= len(seeds) and all(x > math.log(1.25) for x in v): print("  遅", p, [round(math.exp(x), 2) for x in v])
        if len(v) >= len(seeds) and all(x < math.log(0.8) for x in v): print("  速", p, [round(math.exp(x), 2) for x in v])
    one = [(p, s) for p in probs for s in seeds if R.get((p, "base", s)) and R.get((p, c, s)) and solved(R[p, "base", s]) != solved(R[p, c, s])]
    print("  片方だけ解けた:", [(p, s, "base" if solved(R[p, "base", s]) else c) for p, s in one])
print("\n種 1・2 の時間の比 |log| の平均 (種の間の揺れ、両方の種がある問題):")
for c in CFGS:
    v = [abs(math.log((t(R[p, c, 1]) + 1) / (t(R[p, c, 2]) + 1))) for p in probs if (p, c, 1) in R and (p, c, 2) in R]
    if v: print(f"  {c}: {math.exp(sum(v) / len(v)):.3f} 倍 ({len(v)} 問)")
for c in CFGS:
    m = [r["max_rss_mb"] for r in rows if r["cfg"] == c]
    if m: print(c, "最大 RSS", max(m), "MB")
