"""bench.py の結果をまとめる: python summ.py  (問題ごとに def / r / fr の時間 (時間切れは上限) とノード、スレッド別の幾何平均の比)"""
import json, math
from collections import defaultdict
R = defaultdict(dict)
for l in open("res.jsonl"):
    r = json.loads(l)
    if not r["tag"].startswith("b_"): continue
    x = r["result"]; c = r["tag"][2:]
    t = x["time"] if x.get("status") == "optimal" else r["tl"]
    R[(r["threads"], r["problem"], r["seed"])][c] = (t, x.get("nodes"), x.get("status"), x.get("root_bound"))
for thr in (1, 4):
    print(f"== {thr} threads")
    lr = {"r": [], "fr": []}
    for (th, p, s), d in sorted(R.items()):
        if th != thr or "def" not in d: continue
        line = f"{p:15s} s{s} " + " | ".join(f"{c} {d[c][0]:6.1f}s {d[c][1]:>6} {'' if d[c][2]=='optimal' else d[c][2]}" for c in ("def", "r", "fr") if c in d)
        print(line)
        for c in lr:
            if c in d: lr[c].append(math.log((d[c][0] + 1) / (d["def"][0] + 1)))
    for c, v in lr.items():
        if v: print(f"  {c}/def shifted geomean (+1s): {math.exp(sum(v)/len(v)):.3f} over {len(v)}")
