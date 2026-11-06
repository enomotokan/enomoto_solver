"""b300.jsonl などから base と各設定を問題ごとに並べる。使い方: python cmp.py 出力名 [設定...]"""
import json, sys
from pathlib import Path
from collections import defaultdict
D = Path(__file__).resolve().parent
out = sys.argv[1] if len(sys.argv) > 1 else "b300"
cfgs = sys.argv[2:] or ["base", "fix"]
R = defaultdict(dict)
for l in open(D / f"{out}.jsonl"):
    r = json.loads(l); R[r["problem"]][(r["cfg"], r["rep"])] = r
def gap(res):
    o, b = res.get("objective"), res.get("bound")
    if o is None or b is None: return None
    return abs(o - b) / max(abs(o), 1e-9) * 100
def fmt(r):
    if r is None: return "-"
    res, inf = r["result"], r["info"]
    st = {"optimal": "opt", "time_limit": "TL", "infeasible": "inf"}.get(res.get("status"), res.get("status"))
    t = f'{res.get("time", 0):.0f}s' if st == "opt" else ""
    g = gap(res)
    lp = inf["root_lp_s"]
    return f'{st}{t} LP {lp if lp is not None else "-"}s rb {res.get("root_bound") if res.get("root_bound") is None else format(res["root_bound"], ".7g")} obj {res.get("objective") if res.get("objective") is None else format(res["objective"], ".7g")} gap {"-" if g is None else f"{g:.2f}%"}'
print("| 問題 | 回 | " + " | ".join(cfgs) + " |")
print("|---|---|" + "---|" * len(cfgs))
for p, d in R.items():
    for rep in sorted({k[1] for k in d}):
        print(f"| {p} | {rep} | " + " | ".join(fmt(d.get((c, rep))) for c in cfgs) + " |")
