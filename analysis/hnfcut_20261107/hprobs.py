"""HNFCUT の比べに使う問題 (2026-11-07)。HNFPRE の記録 (analysis/hnfpre_20261107.md §2) の「合同式が出る」問題。"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "cbcimpl_20261029"))
# 合同式が上下限を詰めない (分枝・伝播・カットにだけ使う) 問題
CONG = ["brazil3", "mzzv11", "neos-3656078-kumeu", "piperout-08", "piperout-27", "radiationm18-12-05", "radiationm40-10-02", "rocI-4-11",
        "rococoB10-011000", "rococoC10-001000", "snp-02-004-104", "traininstance2", "traininstance6", "lectsched-5-obj", "8div-n59k10", "flugpl"]
# 2 回目 (collection の探索 scan.py で見つけたもの)
HITS2 = ["rococoB10-011000", "rococoC10-001000", "flugpl", "ns1631475"]
# 3 回目 (collection の探索で根の LP 点に合同式に合わない値の列があったもの、と 20 秒で根の LP に届かなかったもの)
HITS3 = ["rococoC12-010001", "neos-3068746-nene", "neos-3373491-avoca", "piperout-d27", "piperout-d20"]
UNREACHED = ["neos-3709489-menik", "neos-3699044-maipo", "neos-3672928-linge", "neos-3214367-sovi", "neos-3654993-kolva", "kosova1",
             "gfd-schedulen180f7d50m30k18", "triptim1", "triptim2", "triptim7", "triptim8"]
SETS = {"cong": CONG, "hits2": HITS2, "hits3": HITS3, "unreached": UNREACHED}
try:
    import probs as _p58
    SETS["p58"] = list(_p58.PROBS) if hasattr(_p58, "PROBS") else list(_p58.probs)
except Exception:
    pass
