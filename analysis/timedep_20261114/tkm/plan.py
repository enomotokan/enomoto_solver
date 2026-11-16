"""分担表: python plan.py → shards.json。4 問 × 種 1〜8 × cfg.py の 5 設定 (160 回)、60 秒・4 スレッド。2 分担。
同じ (問題, 種) の全設定を同じ分担に入れ (機械の差が比に入らない)、種 1〜8 を両方の分担に交互に振る (種 1・3・5・7 と 2・4・6・8)。
種ごとに設定の順をずらす。"""
import json
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
CF = list(CFGS)
PROBS = ["neos-827175", "neos-960392", "10teams", "neos-3083819-nubu"]
shards = [[], []]
for s in range(1, 9):
    for p in PROBS:
        k = s % len(CF)
        shards[(s - 1) % 2] += [[p, c, s, 60] for c in CF[k:] + CF[:k]]
(D / "shards.json").write_text(json.dumps({"shards": shards, "cfgs": CF}, indent=0))
print("shards.json:", len(shards), "分担 ×", [len(x) for x in shards], "回、設定", len(CF), CF)
