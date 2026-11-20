"""分担表を作る: python plan.py SET 案の設定 分担の数 (SET は R・P・M)。R = 4 スレッドの悪化の確かめ (python plan.py R v019 4)。
P = MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, 案}、60 秒。
M = VUBAGG が働く問題 (vubcount と g1 の scan) × 種 1・2 × {def, 案}、600 秒。
同じ (問題, 種) の 2 回は同じ分担に続けて置き、順は種で入れ替える。重さは時間の上限 x 2 で見積もり、貪欲に分ける。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1] / "analysis/cbcimpl_20261029"))
SET, cand, k = sys.argv[1], sys.argv[2], int(sys.argv[3])
if SET == "R":
    # 4 スレッドの悪化が本物かの確かめ (final の M で種 1 だけ遅かった問題と、P で遅かった noswot・neos5・qap10 の重い側を 600 秒で)
    probs, seeds, tl = ["n2seq36q", "wachplan", "piperout-27", "qap10", "fastxgemm-n2r6s0t2"], (2, 3), 600
elif SET == "P":
    from probs import PROBS
    probs, seeds, tl = PROBS, (1, 2, 3), 60
else:
    probs, seeds, tl = ["physiciansched6-2", "piperout-27", "neos8", "neos-4722843-widden", "neos-787933"], (1, 2), 600
jobs = []
for p in probs:
    for s in seeds:
        cf = ["def", cand] if s % 2 else [cand, "def"]
        jobs.append([[p, c, s, tl] for c in cf])
shards = [[] for _ in range(k)]
load = [0] * k
for j in jobs:
    i = load.index(min(load)); shards[i] += j; load[i] += 2 * tl
json.dump({"shards": shards}, open(D / f"shards_{SET}.json", "w"), indent=0)
print(SET, "分担", k, "各", [round(x / 3600, 2) for x in load], "時間 (上限)")
