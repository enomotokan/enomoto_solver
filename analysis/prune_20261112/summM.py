"""PRUNE 2 段目 (Mittelmann 600 秒) の集計: 問題ごとの 3 設定の時間と、def との対の比 (shift 1 秒の SGM、時間切れは 600 秒)。"""
import json, glob, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_M_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
ok = lambda r: r["result"].get("status") in ("optimal", "infeasible")
t = lambda r: min(r["result"]["time"], r["tl"]) if ok(r) else r["tl"]
probs = sorted({k[0] for k in R}); CF = ["def", "subfix0", "prune3"]
print(f"行数 {len(rows)}、crash {sum(r['result'].get('status') in (None, 'crash', 'error') for r in rows)}")
print("問題 | " + " | ".join(f"{c} 種1・2 (秒、T=時間切れ、gap%)" for c in CF))
def cell(r):
    if not r: return "-"
    x = r["result"]
    if ok(r): return f"{x['time']:.0f}"
    o, b = x.get("objective"), x.get("bound")
    g = abs(o - b) / max(1e-9, abs(o)) * 100 if isinstance(o, (int, float)) and isinstance(b, (int, float)) else float("nan")
    return f"T({g:.2f})"
for p in probs:
    print(p, "|", " | ".join("・".join(cell(R.get((p, c, s))) for s in (1, 2)) for c in CF))
for c in CF[1:]:
    lr = []; na = nb = 0; bad = []
    for p in probs:
        for s in (1, 2):
            a, b = R.get((p, "def", s)), R.get((p, c, s))
            if not a or not b: continue
            na += ok(a); nb += ok(b); lr.append(math.log((t(b) + 1) / (t(a) + 1)))
            if a["result"].get("status") == "optimal" == b["result"].get("status") and abs(a["result"]["objective"] - b["result"]["objective"]) > 1e-6 * max(1, abs(a["result"]["objective"])): bad.append(p)
    print(f"{c}: 対 {len(lr)}、解けた def {na} → {nb}、SGM {math.exp(sum(lr)/len(lr))-1:+.1%}、食い違い {bad or 0}")
import random
for c in CF[1:]:
    L = collections.defaultdict(list)
    for p in probs:
        for s in (1, 2):
            a, b = R.get((p, "def", s)), R.get((p, c, s))
            if a and b and (ok(a) or ok(b)): L[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(L); g = lambda q: math.exp(sum(x for p in q for x in L[p]) / sum(len(L[p]) for p in q))
    random.seed(0); bs = sorted(g([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"{c}: どちらかが解けた {len(ps)} 問で SGM {g(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}] (問題単位のブートストラップ)")
