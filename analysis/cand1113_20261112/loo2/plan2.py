"""shards_L2.json を作る (追加の確かめ)。分担 0: enlight_hard の種 1・2 で def・cand1113・prune1112 の 5 部品を 1 つずつ外した形。
分担 1: cbs-cta の種 3・4 で def・cand1113・-tenc・-CLQPRB・-utw (種 1・2 で戻りかけた形)。"""
import json
from pathlib import Path
from cfg import CFGS2
sh0 = [["enlight_hard", c, s, 60] for s in (1, 2) for c in ["def", "cand1113"] + list(CFGS2)]
sh1 = [["cbs-cta", c, s, 60] for s in (3, 4) for c in ("def", "cand1113", "-tenc", "-CLQPRB", "-utw")]
(Path(__file__).resolve().parent / "shards_L2.json").write_text(json.dumps({"shards": [sh0, sh1]}))
