"""分担表を作る: python plan.py P 2 / python plan.py M 14。
対象はオービトープのある問題 (analysis/vsscip_slow_20261118/g1/scan.txt で orbitopes が空でないもの)。設定は {def, orr, ostr} (cfg.py)。
P = MIPLIB の 11 問 × 種 1〜3、60 秒。M = Mittelmann の 10 問 × 種 1〜3、600 秒。4 スレッド。
同じ (問題, 種) の 3 回は同じ分担に続けて置き、順は種で回す。重さは時間の上限 x 3 で見積もり、貪欲に分ける。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET, k = sys.argv[1], int(sys.argv[2])
CF = ["def", "orr", "ostr"]
if SET == "P":
    probs, tl = ["graph20-20-1rand", "misc07", "neos-1456979", "neos-827175", "neos-911970", "neos-960392", "neos8", "noswot",
                 "physiciansched6-2", "rout", "sp150x300d"], 60
else:
    probs, tl = ["fast0507", "neos-1456979", "neos-2978193-inde", "neos-3381206-awhea", "neos-873061", "neos-933966", "neos-960392",
                 "sct2", "uccase12", "graph20-20-1rand"], 600
jobs = []
for p in probs:
    for s in (1, 2, 3):
        rot = CF[s % 3:] + CF[:s % 3]
        jobs.append([[p, c, s, tl] for c in rot])
shards = [[] for _ in range(k)]
load = [0] * k
for j in jobs:
    i = load.index(min(load)); shards[i] += j; load[i] += 3 * tl
json.dump({"shards": shards}, open(D / f"shards_{SET}.json", "w"), indent=0)
print(SET, "分担", k, "各", [round(x / 3600, 2) for x in load], "時間 (上限)")
