"""問題ごとの比べ (analysis/rstcrit_20261111.md): python perprob.py 設定
再スタートの回数が既定と違った回のある問題と、3 種とも 10% 超遅い/速い問題を出す (解けない回は 60 秒、シフト 1)。"""
import json, glob, math, sys
from collections import defaultdict
B = sys.argv[1]
R = defaultdict(dict)
for f in sorted(glob.glob("res_s?.jsonl")):
    for r in map(json.loads, open(f)):
        R[(r["problem"], r["rep"])][r["cfg"]] = r
def t(r):
    x = r["result"]
    return x["time"] if x.get("status") in ("optimal", "infeasible") else 60.0
probs = defaultdict(list)
for (p, s), v in R.items():
    if "def" in v and B in v and v[B]["result"].get("status") != "crash":
        a, b = v["def"], v[B]
        probs[p].append((s, t(a), t(b), len(a["restarts"]), len(b["restarts"])))
slow, fast = [], []
print(f"再スタートの回数が違う回のある問題 (種: def 秒/回数 -> {B} 秒/回数)")
for p in sorted(probs):
    xs = sorted(probs[p])
    if any(x[3] != x[4] for x in xs):
        print(f"  {p:22s} " + "  ".join(f"{s}: {a:.1f}/{ra} -> {b:.1f}/{rb}" for s, a, b, ra, rb in xs))
    r = [math.log((b + 1) / (a + 1)) for _, a, b, _, _ in xs]
    if len(r) >= 2 and all(v > math.log(1.1) for v in r):
        slow.append(p)
    if len(r) >= 2 and all(v < -math.log(1.1) for v in r):
        fast.append(p)
print(f"どの種でも 10% 超遅い: {slow}")
print(f"どの種でも 10% 超速い: {fast}")
