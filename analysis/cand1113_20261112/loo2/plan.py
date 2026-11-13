"""shards_L.json を作る。分担 k = 問題 k (種 1・2 を同じ分担で続けて、(問題, 種) ごとに 22 形)。
時間の上限は 60 秒 (cand1113 は 600 秒の回で 10〜37 秒、def は 0.02〜7 秒。上限は cand1113 の 1.5 倍以上)。"""
import json
from pathlib import Path
from cfg import CFGS
P = ["enlight_hard", "neos-1122047", "cbs-cta"]
shards = [[[p, c, s, 60] for s in (1, 2) for c in CFGS] for p in P]
(Path(__file__).resolve().parent / "shards_L.json").write_text(json.dumps({"shards": shards}))
print(len(shards), sum(map(len, shards)))
