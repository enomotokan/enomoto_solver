"""res58.jsonl (run58.py) の集計: 設定ごとの解けた数・SGM (シフト 10 秒、解けなければ 60 秒)、対の比 (def/old) の幾何平均と 95% 区間
(問題で束ねたブートストラップ)、食い違い (最適で目的値が違う)、区間の空な組で列を残した回数・前処理なしの解き直しの回数。python summ58.py"""
import json, math, random
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in (D / "res58.jsonl").read_text().splitlines():
    r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r["result"]
def t(x): return x["time"] if x.get("status") == "optimal" and x.get("time") is not None else 60.0
pairs = sorted({(p, s) for p, c, s in R if (p, "def", s) in R and (p, "old", s) in R})
print("対の数", len(pairs))
for c in ("def", "old"):
    ts = [t(R[(p, c, s)]) for p, s in pairs]
    print(c, "解けた", sum(R[(p, c, s)].get("status") == "optimal" for p, s in pairs), "SGM", round(math.exp(sum(math.log(x + 10) for x in ts) / len(ts)) - 10, 2),
          "残した組", sum(R[(p, c, s)].get("slack_kept", 0) for p, s in pairs), "解き直し", sum(R[(p, c, s)].get("resolve", 0) for p, s in pairs),
          "orig_only", sum(R[(p, c, s)].get("orig_only", 0) for p, s in pairs), "警告", sum(R[(p, c, s)].get("warn", 0) for p, s in pairs))
lr = {}
for p, s in pairs:
    lr.setdefault(p, []).append(math.log((t(R[(p, "def", s)]) + 10) / (t(R[(p, "old", s)]) + 10)))
probs = sorted(lr)
m = lambda ps: sum(sum(lr[p]) for p in ps) / sum(len(lr[p]) for p in ps)
g = random.Random(0); bs = sorted(m([g.choice(probs) for _ in probs]) for _ in range(2000))
print("比 def/old", round(math.exp(m(probs)), 4), "95%", [round(math.exp(bs[50]), 4), round(math.exp(bs[1949]), 4)])
bad = [(p, s, R[(p, "def", s)].get("objective"), R[(p, "old", s)].get("objective")) for p, s in pairs
       if R[(p, "def", s)].get("status") == R[(p, "old", s)].get("status") == "optimal" and abs(R[(p, "def", s)]["objective"] - R[(p, "old", s)]["objective"]) > 1e-6 * max(1, abs(R[(p, "old", s)]["objective"]))]
print("食い違い", bad)
print("どちらかだけ解けた", [(p, s, R[(p, "def", s)].get("status"), R[(p, "old", s)].get("status")) for p, s in pairs if (R[(p, "def", s)].get("status") == "optimal") != (R[(p, "old", s)].get("status") == "optimal")])
big = sorted(((sum(lr[p]) / len(lr[p]), p) for p in probs))
print("def が速い 5", [(p, round(math.exp(v), 2)) for v, p in big[:5]]); print("def が遅い 5", [(p, round(math.exp(v), 2)) for v, p in big[-5:]])
