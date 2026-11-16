"""誤答の切り分け W の集計: python summW.py > summ_W.txt"""
import json, glob
from pathlib import Path
D = Path(__file__).resolve().parent
OPT = 19635558.2440195
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_W_s*.jsonl"))) for l in open(f) if l.strip()]
CF = json.loads((D / "shards_W.json").read_text())["cfgs"]
def bad(x):
    tol = 1e-6 * abs(OPT)
    st = str(x.get("status") or "").lower()
    b = x.get("bound")
    if st == "optimal" and (x.get("objective") or OPT) > OPT + tol: return "最適値を切り落とした"
    if st == "infeasible": return "実行不能と誤答"
    if b is not None and b > OPT + tol: return "下界が最適値を超えた"
    return None
print("行数", len(rows), "(真の最適 %.7f)" % OPT)
for c in CF:
    R = sorted((r for r in rows if r["cfg"] == c), key=lambda r: r["seed"])
    w = [(r["seed"], bad(r["result"]), r["result"].get("objective")) for r in R if bad(r["result"])]
    cells = " ".join(f"s{r['seed']}:{str(r['result'].get('status'))[:4]}/{r['result'].get('time', 0):.0f}" for r in R)
    print(f"{c:22s} 回 {len(R)}  誤答 {len(w)}  {w}\n    {cells}")
