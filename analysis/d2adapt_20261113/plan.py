"""D2 適応版の分担表を作る。python plan.py → shards_M1.json・shards_P1.json・shards_P2.json (それぞれ分担 1 つ、順に回す)
(M1) 測定 1: HIGHSLOSS の 12 問 (analysis/highsloss_20261113/shards.json) × 種 1 × d2B 600 秒。base・d2b は analysis/d2_20261113/res_M_s*.jsonl を使い、
     最適まで解けた 3 問 (kasai・mzzv42z・hypothyroid-k1) だけこの機械で base・d2b も回し直す。
(P1) 測定 1: 58 問で d2b が悪化・改善した 8 問 × 種 1〜3 × {base, d2b, d2B} 60 秒。
(P2) 測定 2: 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1・2 × {base, d2B} 60 秒。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
HL = sorted({p for s in json.loads((WT / "analysis/highsloss_20261113/shards.json").read_text())["shards"] for p, _ in s})
m1 = [[p, "d2B", 1, 600] for p in HL] + [[p, c, 1, 600] for p in ("neos-3627168-kasai", "mzzv42z", "hypothyroid-k1") for c in ("base", "d2b")]
BAD = ["noswot", "roll3000", "physiciansched6-2", "mzzv42z", "eil33-2", "mc11", "hypothyroid-k1", "10teams"]
p1 = [[p, c, s, 60] for s in (1, 2, 3) for p in BAD for c in ("base", "d2b", "d2B")]
p2 = [[p, c, s, 60] for s in (1, 2) for p in PROBS for c in ("base", "d2B")]
for name, sh in (("M1", m1), ("P1", p1), ("P2", p2)):
    (D / f"shards_{name}.json").write_text(json.dumps({"shards": [sh]}, indent=0))
    print(name, len(sh), "回")
