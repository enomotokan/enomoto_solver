"""all58.jsonl の集計: 試作 (ENOMOTO_MIP_SMALL1=wr) が働いた問題で、既定と時間 (解けなければ 60 秒) を並べ、SGM (ずらし 1 秒) を出す。"""
import json, math
from collections import defaultdict
R = defaultdict(dict)
for l in open(__file__.rsplit("/", 1)[0] + "/all58.jsonl" if "/" in __file__ else "all58.jsonl"):
    x = json.loads(l); R[x["problem"]][x["cfg"]] = x
t = lambda r: r["time"] if r.get("status") == "optimal" and r["time"] <= 60 else 60.0
sg = lambda v: math.exp(sum(math.log(a + 1) for a in v) / len(v)) - 1
rows = [(p, t(d["base"]["result"]), t(d["s1wr"]["result"]), d["base"]["result"], d["s1wr"]["result"]) for p, d in R.items() if "base" in d]
rows.sort(key=lambda r: r[2] / r[1])
print(f"{'問題':24s} {'既定':>7s} {'試作':>7s} 比")
for p, a, b, ra, rb in rows:
    note = "" if ra.get("status") == rb.get("status") else f"  ({ra.get('status')} -> {rb.get('status')})"
    print(f"{p:24s} {a:7.2f} {b:7.2f} {b / a:5.2f}{note}")
print(f"働いた {len(rows)} 問: SGM 既定 {sg([r[1] for r in rows]):.3f} 試作 {sg([r[2] for r in rows]):.3f}; 解けた数 {sum(r[3].get('status')=='optimal' for r in rows)} -> {sum(r[4].get('status')=='optimal' for r in rows)}")
both = [r for r in rows if r[1] < 60 and r[2] < 60]
print(f"両方解けた {len(both)} 問: SGM 既定 {sg([r[1] for r in both]):.3f} 試作 {sg([r[2] for r in both]):.3f}")
