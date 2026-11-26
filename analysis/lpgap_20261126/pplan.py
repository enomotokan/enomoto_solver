"""P (analysis/cbcimpl_20261029/probs.py の 58 問) × 種 1〜3 × (def_t1・ths_t1)、60 秒の分担表 shards.json を作る (1 分担)。
同じ (問題, 種) の 2 設定は続けて解き、順は組ごとに入れ替える。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
rows = []
for i, p in enumerate(PROBS):
    for s in (1, 2, 3):
        c = ["def_t1", "ths_t1"] if (i + s) % 2 == 0 else ["ths_t1", "def_t1"]
        rows += [[p, x, s, 60] for x in c]
(D / "shards.json").write_text(json.dumps({"shards": [rows]}))
print(len(rows))
