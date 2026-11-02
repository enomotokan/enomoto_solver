"""b58.jsonl の集計: 解けた数・SGM (シフト 10 秒、未解決は 60 秒)・目的値の食い違い・1.2 倍以上かつ 1 秒以上の差。"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in (D / "b58.jsonl").read_text().splitlines():
    j = json.loads(l); R.setdefault(j["problem"], {})[j["tag"]] = j["result"]
def t(r): return min(r["time"], 60.0) if r["status"] == "optimal" else 60.0
def sgm(ts, s=10.0): return math.exp(sum(math.log(x + s) for x in ts) / len(ts)) - s
ps = [p for p in R if len(R[p]) == 2]
for tag in ("def", "light1"):
    print(tag, "solved", sum(R[p][tag]["status"] == "optimal" for p in ps), "of", len(ps), "SGM %.2f" % sgm([t(R[p][tag]) for p in ps]))
print("差 (1.2 倍以上かつ 1 秒以上):")
for p in ps:
    a, b = t(R[p]["def"]), t(R[p]["light1"])
    oa, ob = R[p]["def"].get("objective"), R[p]["light1"].get("objective")
    mis = R[p]["def"]["status"] == R[p]["light1"]["status"] == "optimal" and abs(oa - ob) > 1e-6 * max(1, abs(oa))
    if (max(a, b) > 1.2 * min(a, b) and abs(a - b) > 1) or mis:
        print(f"  {p:22s} def {a:6.2f} light1 {b:6.2f}" + ("  目的値の食い違い %s %s" % (oa, ob) if mis else ""))
