"""分担表: python plan.py → shards.json。4 問 × 種 1〜8 × cfg.py の 4 設定 (128 回)、60 秒・4 スレッド。
1 分担 = 種 4 つ (64 回)、2 分担。同じ (問題, 種) の設定は同じ分担に入れ、種ごとに設定の順をずらす。"""
import json
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
CF = list(CFGS)
PROBS = ["neos-827175", "neos-960392", "10teams", "neos-3083819-nubu"]
shards = []
for seeds in ((1, 2, 3, 4), (5, 6, 7, 8)):
    sh = []
    for s in seeds:
        for p in PROBS:
            k = s % len(CF)
            sh += [[p, c, s, 60] for c in CF[k:] + CF[:k]]
    shards.append(sh)
(D / "shards.json").write_text(json.dumps({"shards": shards, "cfgs": CF}, indent=0))
print("shards.json:", len(shards), "分担 ×", len(shards[0]), "回、設定", len(CF), CF)
