"""掃引の集計: python tool/sweep_summ.py [札,...]
基準は選別の回 (tool/screenT.jsonl の札 a = mplwnNebtyc の 1×)。各札について、54 問の手間 (lp_iterations、shift 1000) と時間 (shift 1 秒) の SGM 比、
解けた数、手間が 1.5 倍以上・2/3 以下に変わった問題を出す。時間切れ (手間の上限) は上限の手間で数える。害の出た問題 (4 スレッドの 58 問) は個別に並べる。"""
import json, math, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
base = {r["problem"]: r for l in open(D / "screenT.jsonl") for r in [json.loads(l)] if r["tag"] == "a"}
S = json.loads((D / "S_T.json").read_text())
rows = collections.defaultdict(dict)
for l in open(D / "sweep.jsonl"):
    r = json.loads(l); rows[r["tag"]][r["problem"]] = r
tags = sys.argv[1].split(",") if len(sys.argv) > 1 else sorted(rows)
HARM = ["hypothyroid-k1", "qnet1", "eil33-2", "mas76", "noswot", "10teams", "mc11", "nw04", "neos-911970"]
def solved(r): return r.get("status") in ("optimal", "infeasible")
def work(r): return r.get("lp_iterations") or 0
for t in tags:
    R = rows[t]; ps = [p for p in S if p in R and p in base]
    lw = [math.log((work(R[p]) + 1000) / (work(base[p]) + 1000)) for p in ps]
    lt = [math.log((R[p]["time"] + 1) / (base[p]["time"] + 1)) for p in ps]
    ns = (sum(solved(base[p]) for p in ps), sum(solved(R[p]) for p in ps))
    up = [(p, round((work(R[p]) + 1000) / (work(base[p]) + 1000), 2)) for p, x in zip(ps, lw) if x > math.log(1.5)]
    dn = [(p, round((work(R[p]) + 1000) / (work(base[p]) + 1000), 2)) for p, x in zip(ps, lw) if x < math.log(2 / 3)]
    lost = [p for p in ps if solved(base[p]) and not solved(R[p])]; won = [p for p in ps if solved(R[p]) and not solved(base[p])]
    print(f"\n## {t}: {len(ps)} 問、手間 {math.exp(sum(lw) / len(lw)) - 1:+.1%}、時間 {math.exp(sum(lt) / len(lt)) - 1:+.1%}、解けた {ns[0]} → {ns[1]} (落とす {lost}、得る {won})")
    print("  手間 ×1.5 以上:", up); print("  手間 ×2/3 以下:", dn)
    print("  害の出た問題 (手間の比 / 状態):", [(p, round((work(R[p]) + 1000) / (work(base[p]) + 1000), 2), R[p]["status"][:3]) for p in HARM if p in R and p in base])
