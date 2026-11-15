"""res_P.jsonl と res_X.jsonl (あれば) の対の比較: python summ.py [基準] [比べる設定] (既定 c14 c14u)。SGM は shift 1 秒、時間切れは 60 秒で数える。CI は問題単位のブートストラップ (4000 回)。
「揃って遅い/速い」はその問題の全種で比 > 1.25 / < 0.8。"""
import json, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
A, B = (sys.argv[1], sys.argv[2]) if len(sys.argv) > 2 else ("c14", "c14u")
rows = [json.loads(l) for f in ("res_P.jsonl", "res_X.jsonl") if (D / f).exists() for l in open(D / f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def ok(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if ok(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
logr = collections.defaultdict(list); ns = {A: 0, B: 0}; crash = 0; bad = []; npair = 0
for p in probs:
    for s in seeds:
        a, b = R.get((p, A, s)), R.get((p, B, s))
        if not a or not b: continue
        npair += 1
        for x in (a, b):
            if x["result"].get("status") in (None, "crash", "error"): crash += 1
        ns[A] += ok(a); ns[B] += ok(b)
        if ok(a) and ok(b):
            if a["result"]["status"] != b["result"]["status"]: bad.append((p, s, a["result"]["status"], b["result"]["status"]))
            elif a["result"]["status"] == "optimal":
                oa, ob = a["result"]["objective"], b["result"]["objective"]
                if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
        if ok(a) or ok(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
def sgm(ps):
    v = [x for p in ps for x in logr[p]]
    return math.exp(sum(v) / len(v)) if v else 1.0
ps = list(logr); est = sgm(ps); random.seed(0)
bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"{B} / {A}: 対 {npair}、crash {crash}、食い違い {len(bad)} {bad}")
print(f"解けた数: {A} {ns[A]} / {B} {ns[B]}")
print(f"対の比 (どちらかが解けた {len(ps)} 問): {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
for p in ps:
    v = logr[p]
    if len(v) >= 2 and all(x > math.log(1.25) for x in v): print("  揃って遅い", p, [round(math.exp(x), 2) for x in v], [round(t(R[(p, c, s)]), 1) for s in seeds for c in (A, B) if (p, c, s) in R])
    if len(v) >= 2 and all(x < math.log(0.8) for x in v): print("  揃って速い", p, [round(math.exp(x), 2) for x in v], [round(t(R[(p, c, s)]), 1) for s in seeds for c in (A, B) if (p, c, s) in R])
