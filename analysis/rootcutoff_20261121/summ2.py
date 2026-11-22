"""bench2.py full の結果をまとめる: python summ2.py (問題ごとの種 1〜3 の時間 (時間切れは上限) と、ずらした幾何平均 (+1 秒) の def 比)"""
import json, math
from collections import defaultdict
from pathlib import Path
R = defaultdict(dict)
for l in open(Path(__file__).parent / "res.jsonl"):
    r = json.loads(l)
    if not r["tag"].startswith("f2_"): continue
    x = r["result"]; t = x["time"] if x.get("status") == "optimal" else r["tl"]
    R[(r["problem"], r["seed"])][r["tag"][3:]] = (t, x.get("nodes"), x.get("status"))
probs = sorted({p for p, _ in R})
lr = defaultdict(list)
for p in probs:
    row = []
    for c in ("def", "n", "k"):
        ts = [R[(p, s)][c][0] for s in (1, 2, 3) if c in R[(p, s)]]
        row.append(f"{c} " + "/".join(f"{t:.1f}" for t in ts))
    print(f"{p:15s} " + " | ".join(row))
    for s in (1, 2, 3):
        d = R[(p, s)]
        for c in ("n", "k"):
            if c in d and "def" in d: lr[c].append(math.log((d[c][0] + 1) / (d["def"][0] + 1)))
for c, v in lr.items():
    print(f"{c}/def shifted geomean (+1s): {math.exp(sum(v) / len(v)):.3f} over {len(v)}")
