"""分担表 shards.json を作る: python plan.py。P の 58 問 × 種 1〜3 × 60 秒。
分担 0〜2 = 1 スレッド (def_t1・abc_t1 を (問題, 種) ごとに隣り合わせ、3 つ並べて回す)、分担 3 = 4 スレッド (def_t4・abc_t4、ほかと並べない)。
単独の部品 (a・b・c) は害が出たときだけ別の分担表で足す。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1] / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
t1 = [(p, c, s, 60) for s in (1, 2, 3) for p in PROBS for c in ("def_t1", "abc_t1")]
pairs = [t1[i:i + 2] for i in range(0, len(t1), 2)]
sh = [[x for k, pr in enumerate(pairs) if k % 3 == i for x in pr] for i in range(3)]
sh.append([(p, c, s, 60) for s in (1, 2, 3) for p in PROBS for c in ("def_t4", "abc_t4")])
(D / "shards.json").write_text(json.dumps({"shards": sh}))
print([len(x) for x in sh])
