"""cand1112 の Mittelmann 186 問 × 種 1・2 (600 秒) の対の比較と HiGHS との比べ。
SGM は shift 10 秒 (Mittelmann の慣例、cmp600i.py と同じ)、時間切れは 600 秒。CI は問題単位のブートストラップ。"""
import json, glob, math, random, collections
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600.0
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_M_s*.jsonl"))) for l in open(f)]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
H = {}
for f in glob.glob(str(D.parent / "mitt600_20261103" / "res_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["solver"] == "highs": H[r["problem"]] = r
def st(r): return str(r["result"].get("status", "")).lower()
def solved(r): return (st(r).startswith("optimal") or st(r).startswith("infeasible")) and (r["result"].get("time") or 1e9) <= TL + 5
def t(r): return min(r["result"]["time"], TL) if solved(r) else TL
hs = {p for p, r in H.items() if solved(r)}
probs = sorted({p for p, c, s in R}); seeds = [1, 2]
crash = sum(1 for r in rows if st(r) in ("crash", "error", "") or "error" in r["result"])
ns = collections.Counter(); ho = collections.Counter(); eo = collections.Counter(); bad = []
logr = collections.defaultdict(list); pair = 0
for p in probs:
    for s in seeds:
        a, b = R.get((p, "def", s)), R.get((p, "cand1112", s))
        if not a or not b: continue
        pair += 1
        for c, x in (("def", a), ("cand1112", b)):
            ns[c] += solved(x)
            if p in hs and not solved(x): ho[c] += 1
            if p not in hs and solved(x): eo[c] += 1
        oa, ob = a["result"].get("objective"), b["result"].get("objective")
        if st(a).startswith("optimal") and st(b).startswith("optimal") and oa is not None and ob is not None and abs(oa - ob) > 1e-6 * max(1, abs(oa)):
            bad.append((p, s, oa, ob))
        for x, y in ((a, b), (b, a)):
            ba, ob2 = x["result"].get("bound"), y["result"].get("objective")
            if solved(x) is False and ba is not None and ob2 is not None and solved(y) and math.isfinite(ba) and ba > ob2 + 1e-6 * max(1, abs(ob2)):
                bad.append((p, s, "bound>opt", ba, ob2))
        if solved(a) or solved(b):
            logr[p].append(math.log((t(b) + 10) / (t(a) + 10)))
def sgm_all(c): return math.exp(sum(math.log(t(R[(p, c, s)]) + 10) for p in probs for s in seeds if (p, c, s) in R) / pair) - 10
def ratio(ps):
    v = [x for p in ps for x in logr[p]]
    return math.exp(sum(v) / len(v))
ps = list(logr); random.seed(0)
bs = sorted(ratio([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"行数 {len(rows)} (対 {pair})、crash {crash}、食い違い {len(bad)} {bad}")
print(f"解けた (372 回): def {ns['def']} / cand1112 {ns['cand1112']}")
print(f"SGM (shift 10、全 372 回): def {sgm_all('def'):.2f} / cand1112 {sgm_all('cand1112'):.2f}")
print(f"対の比 (どちらかが解けた {len(ps)} 問): {ratio(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
print(f"HiGHS だけが解く (延べ): def {ho['def']} / cand1112 {ho['cand1112']}、ENOMOTO だけが解く (延べ): def {eo['def']} / cand1112 {eo['cand1112']}")
print("片方だけ解けた (問題 種: def 秒 / cand1112 秒):")
for p in probs:
    for s in seeds:
        a, b = R.get((p, "def", s)), R.get((p, "cand1112", s))
        if a and b and solved(a) != solved(b):
            print(f"  {p} {s}: {t(a):.0f} / {t(b):.0f}")
print("2 種とも 1.25 倍以上遅い / 0.8 倍以下に速い:")
for p in ps:
    v = logr[p]
    if len(v) == 2 and all(x > math.log(1.25) for x in v): print("  遅", p, [round(math.exp(x), 2) for x in v])
    if len(v) == 2 and all(x < math.log(0.8) for x in v): print("  速", p, [round(math.exp(x), 2) for x in v])
for c in ("def", "cand1112"):
    m = [r["max_rss_mb"] for r in rows if r["cfg"] == c and r.get("max_rss_mb")]
    print(c, "最大 RSS", max(m), "MB、5 GB 超", sum(x > 5000 for x in m))
