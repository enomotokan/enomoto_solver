"""PARALLEL_INT の比較の集計: python summ.py SET (D か P)。比 = pint / cand1114 の時間 (shift 1 秒、時間切れは 60 秒)。
SGM の 95% 区間は問題単位のブートストラップ。問題ごとに種別の時間と状態を出す (D は全部、P は差の大きいものと D の 8 問)。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1]
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
DUP = ["nw04", "physiciansched6-2", "mzzv42z", "air03", "mas74", "mas76", "swath1", "neos-960392"]
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
logr = collections.defaultdict(list); ns = {"cand1114": 0, "pint": 0}; bad = []; crash = 0; pairs = 0
for p in probs:
    for s in seeds:
        a, b = R.get((p, "cand1114", s)), R.get((p, "pint", s))
        if not a or not b: continue
        pairs += 1
        for x in (a, b):
            if x["result"].get("status") in (None, "crash", "error", "fetch_error"): crash += 1
        ns["cand1114"] += solved(a); ns["pint"] += solved(b)
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
print(f"行数 {len(rows)}、対 {pairs}、crash {crash}、食い違い {len(bad)} {bad}")
print(f"解けた数: cand1114 {ns['cand1114']} / pint {ns['pint']}")
print(f"SGM の比 pint / cand1114 (どちらかが解けた {len(ps)} 問): {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
print("種をまたいで揃って遅い/速い問題 (全種で比 >1.25 / <0.8):")
for p in ps:
    v = logr[p]
    if len(v) >= 2 and all(x > math.log(1.25) for x in v): print("  遅", p, [round(math.exp(x), 2) for x in v])
    if len(v) >= 2 and all(x < math.log(0.8) for x in v): print("  速", p, [round(math.exp(x), 2) for x in v])
def cell(r):
    if not r: return "-"
    st = r["result"].get("status")
    return f"{r['result']['time']:.1f}" if solved(r) else ("TL" if st == "time_limit" else str(st))
show = DUP if SET == "D" else sorted(set(p for p in ps if any(abs(x) > math.log(1.5) for x in logr[p])) | set(p for p in DUP if p in probs))
print("問題ごと (秒、種 " + " / ".join(map(str, seeds)) + "): cand1114 → pint、比")
for p in show:
    print(f"  {p}: " + " / ".join(cell(R.get((p, 'cand1114', s))) for s in seeds) + " → " + " / ".join(cell(R.get((p, 'pint', s))) for s in seeds)
          + "  比 " + " ".join(f"{math.exp(x):.2f}" for x in logr.get(p, [])))
