"""shards_L2.json を作る (追加の確かめ)。neos-960392 の種 1〜4 で def・cand1113・-prune1112・prune1112 の 5 部品を 1 つずつ外した形、
binkar10_1 の種 3・4 で def・cand1113・-DUALEQ。(問題, 種) ごとに続けて回す。"""
import json
from pathlib import Path
from cfg import CFGS
NC = ["def", "cand1113", "-prune1112"] + [c for c in CFGS if c.startswith("-P:")]
shards = [[["neos-960392", c, s, 60] for c in NC] for s in (1, 2, 3, 4)]
shards += [[["binkar10_1", c, s, 60] for c in ("def", "cand1113", "-DUALEQ")] for s in (3, 4)]
(Path(__file__).resolve().parent / "shards_L2.json").write_text(json.dumps({"shards": shards}))
print(len(shards), sum(map(len, shards)))
