"""仕上げの対のまとめ: python summ.py (res_s*.jsonl を読み、全体と問題ごとの cand1114 対 pgq を表にする)"""
import json, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for f in sorted(D.glob("res_s*.jsonl")):
    for l in f.read_text().splitlines():
        if l.strip():
            r = json.loads(l); R[r["cfg"]][(r["problem"], r["seed"])] = r["result"]
def tt(res): return min(res.get("time") or 60, 60) if res.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 1) for x in xs) / len(xs)) - 1
A, B = R["cand1114"], R["pgq"]; ks = sorted(set(A) & set(B))
bad = [(k, c) for c, X in R.items() for k, v in X.items() if v.get("status") not in ("optimal", "time_limit")]
objs = collections.defaultdict(set)
for X in R.values():
    for k, v in X.items():
        if v.get("status") == "optimal": objs[k[0]].add(round(v["objective"], 6))
diff = [p for p, o in objs.items() if max(o) - min(o) > 1e-6 * max(1, max(abs(x) for x in o))]
print(f"回数 cand1114 {len(A)}・pgq {len(B)}、組 {len(ks)}、crash など {len(bad)}、目的値の食い違い {diff or 'なし'}")
for name, X in (("cand1114", A), ("pgq", B)):
    print(f"{name}: 解けた {sum(X[k].get('status') == 'optimal' for k in ks)}、sgm1 {sgm([tt(X[k]) for k in ks]):.2f}")
both = [k for k in ks if A[k].get("status") == "optimal" or B[k].get("status") == "optimal"]
lr = [math.log((tt(B[k]) + 1) / (tt(A[k]) + 1)) for k in both]
m = sum(lr) / len(lr); sd = (sum((x - m) ** 2 for x in lr) / (len(lr) - 1)) ** .5
print(f"どちらか解けた {len(both)} 組の sgm1 比 (pgq/cand1114) {math.exp(m):.3f}、対数比の標準偏差 {sd:.3f}、標準誤差 {sd / len(lr) ** .5:.3f}")
print("\n| 問題 | cand1114 (種 1〜4) | pgq (種 1〜4) | 比 |\n|---|---|---|---|")
for p in sorted({k[0] for k in ks}):
    a = [tt(A[(p, s)]) for s in range(1, 5) if (p, s) in A]; b = [tt(B[(p, s)]) for s in range(1, 5) if (p, s) in B]
    f = lambda xs, X: " ".join(("T" if X[(p, s)].get("status") != "optimal" else f"{x:.1f}") for s, x in zip(range(1, 5), xs))
    print(f"| {p} | {f(a, A)} | {f(b, B)} | {(sgm(b) + 1) / (sgm(a) + 1):.2f} |")
