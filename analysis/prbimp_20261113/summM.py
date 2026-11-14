"""PRBIMP: 計測 M (と T・T2) の表。時間切れの問題は解と下界も並べる。python summM.py > summM_table.txt"""
import glob, json
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
for f in sorted(glob.glob(str(D / "res_M_s*.jsonl"))) + [str(D / "res_T_s0.jsonl"), str(D / "res_T2_s0.jsonl")]:
    for l in open(f):
        r = json.loads(l); rows[(r["problem"], r["seed"], r["cfg"])] = r["result"]
def cell(x):
    if x is None: return "-"
    st = x.get("status")
    if st == "optimal": return f"{x['time']:.1f} 秒"
    if st == "time_limit": return f"時間切れ 解 {x.get('objective')} 下界 {x.get('bound'):.6g}" if x.get("bound") is not None else f"時間切れ 解 {x.get('objective')}"
    return str(st)
ps = sorted({k[0] for k in rows})
for p in ps:
    for s in sorted({k[1] for k in rows if k[0] == p}):
        a, b = rows.get((p, s, "c14")), rows.get((p, s, "c14clqg"))
        print(f"| {p} | {s} | {cell(a)} | {cell(b)} |")
