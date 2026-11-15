"""b58.jsonl の集計: 解けた数・SGM (ずらし 10 秒、解けなければ 60 秒)・目的値の食い違い・時間の比が大きい問題"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "b58.jsonl"):
    r = json.loads(l); R.setdefault(r["problem"], {})[r["tag"]] = r["result"]
def solved(x): return x["status"] in ("optimal", "infeasible") and x["time"] <= 65
def t(x): return x["time"] if solved(x) else 60.0
for tag in ("def", "trial"):
    xs = [R[p][tag] for p in R]
    print(tag, "解けた", sum(solved(x) for x in xs), "SGM", round(math.exp(sum(math.log(t(x) + 10) for x in xs) / len(xs)) - 10, 2))
print("食い違い / 片方だけ / 時間の比 (trial/def) が 0.7 未満か 1.4 超:")
for p, v in R.items():
    a, b = v["def"], v["trial"]
    if solved(a) and solved(b) and abs(a["objective"] - b["objective"]) > 1e-6 * max(1, abs(a["objective"])):
        print("  食い違い", p, a["objective"], b["objective"])
    if solved(a) != solved(b):
        print("  片方だけ", p, a["status"], round(a["time"], 1), "->", b["status"], round(b["time"], 1), "gap", a.get("objective"), a.get("bound"), b.get("objective"), b.get("bound"))
    elif solved(a):
        q = (t(b) + 10) / (t(a) + 10)
        if q < 0.7 or q > 1.4:
            print(f"  {p}: {a['time']:.1f} -> {b['time']:.1f}")
