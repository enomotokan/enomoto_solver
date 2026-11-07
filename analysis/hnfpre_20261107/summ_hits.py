"""hits*.jsonl の表 (問題ごとに設定 × 回の状態と時間)。使い方: python summ_hits.py hits60.jsonl [T]"""
import json, sys, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
f = Path(sys.argv[1]); f = f if f.exists() else D / sys.argv[1]
T = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
R = defaultdict(dict); order = []
for l in f.open():
    r = json.loads(l)
    if r["problem"] not in order: order.append(r["problem"])
    R[r["problem"]][(r["cfg"], r["rep"])] = r["result"]
cfgs = sorted({c for p in R for c, _ in R[p]}, key=lambda c: (c != "def", c))
reps = sorted({s for p in R for _, s in R[p]})
def cell(res):
    if res is None: return "-"
    st = res.get("status"); t = res.get("time") or 0
    if st in ("optimal", "infeasible") and t <= T: return f"{t:.2f}{'' if st == 'optimal' else ' (inf)'}"
    return f"時間切れ" if st in ("time_limit", "timelimit", "TimeLimit") or t >= T else str(st)
def solved(res): return res and res.get("status") in ("optimal", "infeasible") and (res.get("time") or 0) <= T
print("| 問題 | " + " | ".join(f"{c} 回{s}" for c in cfgs for s in reps) + " |")
print("|---|" + "---|" * (len(cfgs) * len(reps)))
for p in order:
    print(f"| {p} | " + " | ".join(cell(R[p].get((c, s))) for c in cfgs for s in reps) + " |")
for c in cfgs:
    ts = [min(R[p][(c, s)].get("time") or T, T) if solved(R[p].get((c, s))) else T for p in order for s in reps if (c, s) in R[p]]
    n = sum(1 for p in order for s in reps if solved(R[p].get((c, s))))
    sgm = math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10 if ts else float('nan')
    print(f"\n{c}: 解けた回 {n}/{len(ts)}、SGM(ずらし 10) {sgm:.2f}", end="")
print()
