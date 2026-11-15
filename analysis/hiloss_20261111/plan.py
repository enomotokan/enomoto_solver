"""HILOSS の分担表 shards.json を作る: losses.txt の A・B・C の 26 問を、見積り (ENOMOTO 600 秒 + HiGHS の res の時間) で NS 個に釣り合うよう分ける。
使い方: python plan.py NS"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
NS = int(sys.argv[1])
est = {}
for l in (D / "losses.txt").read_text().splitlines():
    w = l.split()
    if len(w) > 5 and w[0] in "ABC" and len(w[0]) == 1:
        e = [600 if x == "TL" else float(x) for x in w[3:5]]
        est[w[1]] = float(w[2]) + max(e) + 40
load_ = [0.0] * NS; shards = [[] for _ in range(NS)]
for p in sorted(est, key=lambda p: -est[p]):
    i = min(range(NS), key=lambda j: load_[j]); shards[i].append(p); load_[i] += est[p]
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load_]}, indent=0))
print(len(est), "問", [round(x / 3600, 2) for x in load_], shards)
