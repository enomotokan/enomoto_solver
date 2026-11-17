"""分担表を作る: python plan.py → shards.json。60 秒の 3 問 × 種 1〜4 (同じ (問題, 種) の 15 設定は同じ分担、4 分担) と、
tbfp-network (600 秒) × 種 1・2 (15 設定を 3 つに分け、各組に full と -all を入れる。6 分担)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent; sys.path.insert(0, str(D))
from cfg import CFGS, P60, P600  # noqa: E402
names = list(CFGS)
pairs = [(p, s) for p in P60 for s in (1, 2, 3, 4)]
shards = [[[p, c, s, 60] for p, s in pairs[k::4] for c in names] for k in range(4)]
rest = [c for c in names if c not in ("full", "-all")]
for p in P600:
    for s in (1, 2):
        for g in range(3):
            shards.append([[p, c, s, 600] for c in ["full", "-all"] + rest[g::3]])
(D / "shards.json").write_text(json.dumps({"shards": shards}, indent=0))
print(len(shards), "分担、回数", [len(x) for x in shards])
