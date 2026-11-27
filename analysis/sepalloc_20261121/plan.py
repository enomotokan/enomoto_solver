"""通しの計測の分担表を作る: python plan.py P 6 / python plan.py M 12。設定は {def, sa} (cfg.py)。4 スレッド・種 1〜3。
P = MIPLIB の 58 問 (analysis/vsscip_20261120/shards_P.json と同じ問題)、60 秒。
M = Mittelmann の問題のうち、根だけの比べ (cmp_a2.txt) で a2 が閉じた割合を 1 ポイント以上伸ばした 12 問、600 秒
(1 ポイントは計測する問題を選ぶための線で、ソルバーの判断には使わない)。
同じ (問題, 種) の 2 回は同じ分担に続けて置き、順は種で入れ替える。重さは時間の上限 × 2 で見積もり、貪欲に分ける。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET, k = sys.argv[1], int(sys.argv[2])
CF = ["def", "sa"]
if SET == "P":
    sh = json.loads((D.parent / "vsscip_20261120/shards_P.json").read_text())["shards"]
    probs, tl = sorted({t[0] for s in sh for t in s}), 60
else:
    probs, tl = ["beasleyC3", "comp21-2idx", "dano3_3", "germanrr", "icir97_tension", "irp", "mzzv11", "nexp-150-20-8-5", "ns1830653",
                 "sp98ar", "thor50dday", "trento1"], 600
jobs = []
for p in probs:
    for s in (1, 2, 3):
        rot = CF[s % 2:] + CF[:s % 2]
        jobs.append([[p, c, s, tl] for c in rot])
shards = [[] for _ in range(k)]
load = [0] * k
for j in jobs:
    i = load.index(min(load)); shards[i] += j; load[i] += 2 * tl
json.dump({"shards": shards}, open(D / f"shards_{SET}.json", "w"), indent=0)
print(SET, len(probs), "問", "分担", k, "各", [round(x / 3600, 2) for x in load], "時間 (上限)")
