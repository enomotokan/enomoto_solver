"""分担表: python plan.py N → shards.json。5 問 × 種 1〜4 の 60 秒 × 15 設定 (同じ (問題, 種) の 15 設定は同じ分担)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent; sys.path.insert(0, str(D))
from cfg import CFGS, P60  # noqa: E402
N = int(sys.argv[1])
pairs = [(p, s) for p in P60 for s in (1, 2, 3, 4)]
shards = [[[p, c, s, 60] for p, s in pairs[k::N] for c in CFGS] for k in range(N)]
(D / "shards.json").write_text(json.dumps({"shards": shards}, indent=0))
print(N, "分担、回数", [len(x) for x in shards])
