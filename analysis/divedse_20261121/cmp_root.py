"""run_root.py の結果の def と dc の比べ: python cmp_root.py 出力名
問題ごとに、潜りの秒の合計・木に入るまでの秒 (tree start の solve)・根の下界・解・DSE_COST の数を並べる。"""
import json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
for l in open(D / f"{sys.argv[1]}.jsonl"):
    r = json.loads(l); rows.setdefault(r["problem"], {})[r["cfg"]] = r
def ts(r): return r["tree_start"][1] if r.get("tree_start") else r.get("time")
print(f"{'問題':24s} {'潜り def':>9s} {'潜り dc':>9s} {'根 def':>8s} {'根 dc':>8s} {'下界 def':>14s} {'下界 dc':>14s} {'解 def':>14s} {'解 dc':>14s}  skip/built/noest")
rd, rt = [], []; sd = sc = 0.0
for p in sorted(rows):
    a, b = rows[p].get("def"), rows[p].get("dc")
    if not a or not b: continue
    ta, tb = ts(a), ts(b)
    f = lambda v: "-" if v is None else f"{v:.6g}"
    print(f"{p:24s} {a['dive_secs']:9.2f} {b['dive_secs']:9.2f} {ta or 0:8.2f} {tb or 0:8.2f} {f(a['root_bound']):>14s} {f(b['root_bound']):>14s} {f(a['objective']):>14s} {f(b['objective']):>14s}  {b['dse_cost']}")
    sd += a["dive_secs"]; sc += b["dive_secs"]
    if a["dive_secs"] >= 1 and b["dive_secs"] >= 0: rd.append(math.log(max(b["dive_secs"], 0.01) / a["dive_secs"]))
    if ta and tb and ta >= 1: rt.append(math.log(tb / ta))
g = lambda v: math.exp(sum(v) / len(v)) if v else float("nan")
print(f"潜りの秒の合計 def {sd:.1f} dc {sc:.1f}; 潜りが 1 秒以上の {len(rd)} 問で比の幾何平均 {g(rd):.3f}; 根の秒が 1 秒以上の {len(rt)} 問で比の幾何平均 {g(rt):.3f}")
