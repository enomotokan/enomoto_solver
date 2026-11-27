"""probrep の対の比べの P の分担表 (probework_20261120/plan.py の P の部分の写し): python plan.py NP → shards_P.json
P: MIPLIB の 58 問 × 種 1〜3 × {def, pr}、60 秒 (4 スレッド)。同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。
見積り: vshighs_20261117 の def の時間 + 8 秒。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[0]
sys.path.insert(0, str(A / "probework_20261120"))
sys.path.insert(0, str(A / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
import plan as pw  # noqa: E402
pw.D = D
ALT = "pr"
if __name__ == "__main__":
    NP = int(sys.argv[1])
    TP = pw.times(A / "vshighs_20261117/res_P_s*.jsonl", "def", 60)
    def estP(p, c): return TP.get(p, 48.0) + 8
    pw.balance({(p, s): pw.order(p, s, ["def", ALT]) for p in PROBS for s in (1, 2, 3)}, estP, NP, "shards_P.json", 60)
