"""得の 5 問・損の 4 問の SGM (シフト 1 秒、時間切れ・60 秒超は 60 秒) と解けた数。使い方: python sgm.py 設定,..."""
import json, math, sys
from collections import defaultdict
from pathlib import Path
R = defaultdict(dict)
for l in open(Path(__file__).resolve().parent / "res.jsonl"):
    r = json.loads(l)
    ok = r["status"] == "optimal" and r["time"] <= 60
    R[r["config"]].setdefault(r["problem"], []).append((r["time"] if ok else 60.0, ok))
G = ["physiciansched6-2", "qiu", "h80x6320d", "misc07", "nu25-pr12"]
L = ["neos17", "binkar10_1", "swath1", "pk1"]
def sgm(c, ps):
    v = [x for p in ps for x in R[c].get(p, [])]
    return math.exp(sum(math.log(t + 1) for t, _ in v) / len(v)) - 1, sum(o for _, o in v), len(v)
print("| 設定 | 得の 5 問 SGM (解けた/回) | 損の 4 問 SGM (解けた/回) |")
print("|---|---|---|")
for c in sys.argv[1].split(","):
    g, l = sgm(c, G), sgm(c, L)
    print(f"| {c} | {g[0]:.1f} ({g[1]}/{g[2]}) | {l[0]:.1f} ({l[1]}/{l[2]}) |")
