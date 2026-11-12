"""shards_L.json を作る。1 分担 = (問題, 種) ごとに def・cand1113・-X 20 形を続けて。手元 1 本で回すので分担を順に並べる。"""
import json
from pathlib import Path
from cfg import CFGS
P = ["neos-960392", "binkar10_1", "roll3000", "pk1", "10teams", "neos8", "h80x6320d"]
shards = [[[p, c, s, 60] for c in CFGS] for s in (1, 2) for p in P]
(Path(__file__).resolve().parent / "shards_L.json").write_text(json.dumps({"shards": shards}))
print(len(shards), sum(map(len, shards)))
