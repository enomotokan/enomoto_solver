"""=k (=c を整数列だけに) の分担表を作る: python plan_k.py → shards_k.json。
分担 0 = 切り分けの問題 (plan_split.py と同じ) に =k・=abk を種 1〜5 (結果は res_k0.jsonl、ほかと並べず順に)。
分担 1〜3 = P の 58 問 × 種 1〜3 の =abk_t1 (3 つ並べる、既定 def_t1 は res_s0〜s2.jsonl)、分担 4 = 同じく =abk_t4 (ほかと並べない、既定は res_s3.jsonl)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1] / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
s0 = []
for th, probs in (("t4", ["noswot", "10teams", "pk1", "neos-911970"]), ("t1", ["noswot", "pk1"])):
    for p in probs:
        s0 += [(p, f"{c}_{th}", s, 60) for s in (1, 2, 3, 4, 5) for c in ("k", "abk")]
t1 = [(p, "abk_t1", s, 60) for s in (1, 2, 3) for p in PROBS]
sh = [s0] + [t1[i::3] for i in range(3)] + [[(p, "abk_t4", s, 60) for s in (1, 2, 3) for p in PROBS]]
(D / "shards_k.json").write_text(json.dumps({"shards": sh}))
print([len(x) for x in sh])
