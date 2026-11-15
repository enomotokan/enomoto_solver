"""58 問: 両方とも解けた回のノード数の SGM (ずらし 100) と時間の SGM (ずらし 10) の比 (論文の表 5 の形、ノードが 0 の回を除く)。使い方: python nodes58.py"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in open(here / "p58.jsonl"):
    if l.strip():
        r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
ok = lambda x: x.get("status") == "optimal" and x.get("time", 1e9) <= 60
def sgm(v, s): return math.exp(sum(math.log(x + s) for x in v) / len(v)) - s
for B in ["bc", "bcA"]:
    ks = [k for k in R if "def" in R[k] and B in R[k] and ok(R[k]["def"]) and ok(R[k][B]) and max(R[k]["def"]["nodes"], R[k][B]["nodes"]) > 0]
    na = [R[k]["def"]["nodes"] for k in ks]; nb = [R[k][B]["nodes"] for k in ks]
    ta = [R[k]["def"]["time"] for k in ks]; tb = [R[k][B]["time"] for k in ks]
    print(f"{B}: 両方解けて分枝した回 {len(ks)}  ノード SGM {sgm(na,100):.0f} → {sgm(nb,100):.0f} ({sgm(nb,100)/sgm(na,100):.3f} 倍)  時間 SGM {sgm(ta,10):.2f} → {sgm(tb,10):.2f} ({(sgm(tb,10)+10)/(sgm(ta,10)+10):.3f} 倍、ずらし込み)")
