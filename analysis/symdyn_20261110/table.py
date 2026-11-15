"""問題ごとの各回の時間 (時間切れは TL とギャップ) と SYMDYN の数 (締めた境界/刈ったノード): python table.py res.jsonl 設定,設定 [sym]"""
import json, sys
from collections import defaultdict
from pathlib import Path
HERE = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(HERE / sys.argv[1]) if l.strip()]
cfgs = sys.argv[2].split(",")
only = set(sys.argv[3].split(",")) if len(sys.argv) > 3 else None
T = defaultdict(lambda: defaultdict(list))
for r in rows:
    T[r["problem"]][r["cfg"]].append(r)
def cell(r):
    x = r["result"]; s = ""
    if x.get("status") == "optimal" and x.get("time", 1e9) <= r["tl"]:
        s = f"{x['time']:.1f}"
    else:
        o, b = x.get("objective"), x.get("bound")
        g = abs(o - b) / max(1e-9, abs(o)) if o is not None and b is not None and abs(b) < 1e300 else None
        s = "TL" + (f" ({g:.3f})" if g is not None else " (-)")
    d = r.get("sym", {}).get("symdyn")
    if d:
        s += f" [{int(d[-1][2])}/{int(d[-1][4])}]"
    return s
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in T:
    if only and p not in only:
        continue
    print(f"| {p} | " + " | ".join("、".join(cell(r) for r in sorted(T[p][c], key=lambda r: r['rep'])) for c in cfgs) + " |")
