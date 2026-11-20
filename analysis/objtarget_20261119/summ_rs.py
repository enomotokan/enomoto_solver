"""再スタートの費用の試作 (README の 10 節) の集計: python summ_rs.py > summ_RS.txt。
問題・設定ごとに種ごとの (秒, 再スタートの回数) と、shift 1 の幾何平均。食い違い (目的値) も見る。"""
import json, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for f in ["res_RS_s0.jsonl", "res_RS_s1.jsonl"]:
    for l in open(D / f):
        d = json.loads(l); r = d["result"]
        R[(d["problem"], d["cfg"])][d["seed"]] = r
def g(ts): return math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1
for p in sorted({k[0] for k in R}):
    cfgs = [c for (q, c) in R if q == p]
    order = ["def", "c", "cb", "cba", "pl", "pl_c", "pl_cb", "pl_cba", "plw", "plw_cba", "vb", "vb_c", "vb_cb", "vb_cba"]
    objs = set()
    print(p)
    for c in [c for c in order if c in cfgs]:
        v = R[(p, c)]
        ts = [min(v[s]["time"], 60) if v[s]["status"] in ("optimal", "infeasible") else 60 for s in sorted(v)]
        for s in v:
            if v[s]["status"] == "optimal": objs.add(round(v[s]["objective"], 6))
        print(f"  {c:8s} sgm {g(ts):6.1f}  " + "  ".join(f"s{s} {ts[i]:5.1f} r{v[s].get('restarts')} {v[s]['status'][:4]}" for i, s in enumerate(sorted(v))))
    if len(objs) > 1: print("  !! 最適値の食い違い", objs)
