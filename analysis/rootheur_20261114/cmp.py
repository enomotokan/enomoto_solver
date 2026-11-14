"""対の比較: python cmp.py 基準 試作 res_*.jsonl ... (batch.py の res 行 [状態, 目的値, 下界, ノード, 根の下界, 秒]、または runM.py の行)。
SGM は shift 1 秒、時間切れは制限時間で数える。区間は問題単位のブートストラップ (4000 回、95%)。"""
import json, math, random, collections, sys
A, B = sys.argv[1], sys.argv[2]
R = {}
for f in sys.argv[3:]:
    for l in open(f):
        r = json.loads(l)
        if "res" in r:
            st = r["res"][0]; tm = float(r["res"][-1]) if len(r["res"]) >= 6 else None
            ob = None if len(r["res"]) < 2 or r["res"][1] == "None" else float(r["res"][1])
        else:
            st = r["result"].get("status"); tm = r["result"].get("time"); ob = r["result"].get("objective")
        R[r["problem"], r["cfg"], r["seed"]] = (st, tm, ob, r["tl"])
def solved(x): return x[0] in ("optimal", "infeasible")
def t(x): return min(x[1], x[3]) if solved(x) else x[3]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
logr = collections.defaultdict(list); ns = collections.Counter(); bad = []; crash = 0; pairs = 0; diff = []
for p in probs:
    for s in seeds:
        a, b = R.get((p, A, s)), R.get((p, B, s))
        if not a or not b: continue
        pairs += 1
        crash += sum(x[0] not in ("optimal", "infeasible", "time_limit", "node_limit") for x in (a, b))
        ns[A] += solved(a); ns[B] += solved(b)
        if a[0] == b[0] == "optimal" and abs(a[2] - b[2]) > 1e-6 * max(1, abs(a[2])): bad.append((p, s, a[2], b[2]))
        if {a[0], b[0]} == {"optimal", "infeasible"}: bad.append((p, s, a[0], b[0]))
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
        if solved(a) != solved(b): diff.append((p, s, a[0], round(t(a), 1), b[0], round(t(b), 1)))
def sgm(ps):
    v = [x for p in ps for x in logr[p]]
    return math.exp(sum(v) / len(v)) if v else 1.0
ps = list(logr); est = sgm(ps); random.seed(0)
bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"対 {pairs}、crash {crash}、食い違い {len(bad)} {bad}")
print(f"解けた数: {A} {ns[A]} / {B} {ns[B]}")
print(f"対の比 {B}/{A} (どちらかが解けた {len(ps)} 問): {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
print("片側だけ解けた:", diff)
print("種をまたいで揃って遅い/速い (全種で比 >1.25 / <0.8):")
for p in ps:
    v = logr[p]
    if len(v) >= 2 and all(x > math.log(1.25) for x in v): print("  遅", p, [round(math.exp(x), 2) for x in v])
    if len(v) >= 2 and all(x < math.log(0.8) for x in v): print("  速", p, [round(math.exp(x), 2) for x in v])
