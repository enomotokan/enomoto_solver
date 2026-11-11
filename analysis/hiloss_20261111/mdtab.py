"""HILOSS: exp.py と同じデータから、設定の組ごとの markdown の表を出す (記録 analysis/hiloss_20261111.md に貼る用)。使い方: python mdtab.py cq cqr"""
import json, glob, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
base, hi, ex = {}, {}, {}
for f in sorted(glob.glob(str(D / "res_s*.jsonl")), reverse=True):
    for l in open(f):
        r = json.loads(l); (base if r["solver"] == "ours" else hi)[r["problem"]] = r["result"]
for f in sorted(glob.glob(str(D / "res[2345]_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l); ex.setdefault(r["problem"], {})[r["cfg"]] = r["result"]
def g(v): return "-" if v is None or (isinstance(v, float) and not math.isfinite(v)) else f"{v:.8g}"
def cell(x):
    st = str(x.get("status", "")).lower()
    if st.startswith(("optimal", "infeasible")): return f"**{x['time']:.0f} 秒**" + (" (実行不能)" if st.startswith("infeasible") else "")
    return f"TL、解 {g(x.get('objective'))}、下界 {g(x.get('bound'))}"
cfgs = sys.argv[1:]
print("| 問題 | HiGHS | 既定 (根の下界) | " + " | ".join(f"{c} (根の下界)" for c in cfgs) + " |")
print("|---|---|---|" + "---|" * len(cfgs))
for p in sorted(ex):
    if not any(c in ex[p] for c in cfgs): continue
    row = [p, cell(hi[p]), f"{cell(base[p])} ({g(base[p].get('root_bound'))})"]
    for c in cfgs:
        row.append(f"{cell(ex[p][c])} ({g(ex[p][c].get('root_bound'))})" if c in ex[p] else "")
    print("| " + " | ".join(row) + " |")
