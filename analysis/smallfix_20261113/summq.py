"""SMALLFIX の d1g の 58 問 (Q) の集計: python summq.py > summq.txt (summ.py の P の部分と同じ数え方。base / d1g)"""
import json, glob, math, random, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = {(r["problem"], r["cfg"], r["seed"]): r for f in sorted(glob.glob(str(D / "res_Q_s*.jsonl"))) for l in open(f) for r in [json.loads(l)]}
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
print(f"Q: 行数 {len(R)}、問題 {len(probs)}、種 {seeds}、crash {[k for k, r in R.items() if r['result'].get('status') in (None, 'crash', 'error')]}")
logr = collections.defaultdict(list); ns = [0, 0]; bad = []; diff = []
for p in probs:
    for s in seeds:
        a, b = R.get((p, "base", s)), R.get((p, "d1g", s))
        if not a or not b: continue
        ns[0] += solved(a); ns[1] += solved(b)
        sa, sb = a["result"].get("status"), b["result"].get("status")
        if sa == sb == "optimal" and abs(a["result"]["objective"] - b["result"]["objective"]) > 1e-6 * max(1, abs(a["result"]["objective"])): bad.append((p, s))
        if {sa, sb} == {"optimal", "infeasible"}: bad.append((p, s, sa, sb))
        if solved(a) != solved(b): diff.append((p, s, round(t(a), 1), round(t(b), 1)))
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
ps = list(logr)
def sgm(pp):
    v = [x for p in pp for x in logr[p]]
    return math.exp(sum(v) / len(v)) if v else 1.0
est = sgm(ps); random.seed(0)
bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"解けた base {ns[0]} / d1g {ns[1]}、食い違い {len(bad)} {bad}、解けた/解けないが変わった {diff}")
print(f"対の比 (どちらかが解けた {len(ps)} 問): {est - 1:+.1%} [{bs[100] - 1:+.1%}, {bs[3899] - 1:+.1%}]")
for p in ps:
    v = logr[p]
    if len(v) >= 3 and all(x > math.log(1.25) for x in v): print("  揃って遅", p, [round(math.exp(x), 2) for x in v])
    if len(v) >= 3 and all(x < math.log(0.8) for x in v): print("  揃って速", p, [round(math.exp(x), 2) for x in v])
for c in ("base", "d1g"):
    rs = [r for k, r in R.items() if k[1] == c]
    print(f"{c}: 木の中の再スタート {sum(r['cnt']['tree_rst'] for r in rs)} (うち固定 0 列 {sum(r['cnt']['tree_rst_fix0'] for r in rs)})、止めた {sum(r['cnt']['trnofix_stop'] for r in rs)}")
print("d1g で止めた (問題, 種, 回):", sorted((k[0], k[2], r["cnt"]["trnofix_stop"]) for k, r in R.items() if k[1] == "d1g" and r["cnt"]["trnofix_stop"]))
for p in ("supportcase26", "timtab1", "noswot"):
    print(p, " | ".join(f"種 {s}: base {R[p,'base',s]['result']['status'][:4]} {t(R[p,'base',s]):.1f} lb {R[p,'base',s]['result'].get('bound'):.1f} / d1g {R[p,'d1g',s]['result']['status'][:4]} {t(R[p,'d1g',s]):.1f} lb {R[p,'d1g',s]['result'].get('bound'):.1f}" for s in seeds))
