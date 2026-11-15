"""切り分けの分担表 shards_B.json を作る: python plan_B.py
Mittelmann で cand1111 が揃って遅い・解けなくなった問題と、揃って速い問題について、(問題, 種) ごとに def と only_* 9 つを
同じ分担に入れる (対の比較のため)。600 秒・4 スレッド。"""
import json
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
PROBS = ["hypothyroid-k1", "neos859080", "mas74", "beasleyC3", "gen-ip054", "n2seq36q", "neos-3381206-awhea", "tbfp-network",
         "pg5_34", "neos-2978193-inde"]
cfgs = ["def"] + [c for c in CFGS if c.startswith("only_")]
shards = [[[p, c, s, 600] for c in (cfgs if (i + s) % 2 == 0 else cfgs[::-1])] for i, p in enumerate(PROBS) for s in (1, 2)]
(D / "shards_B.json").write_text(json.dumps({"shards": shards, "cfgs": cfgs}))
print(len(shards), "分担、各", len(cfgs), "回")
