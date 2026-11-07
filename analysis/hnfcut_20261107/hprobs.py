"""HNFCUT の比べに使う問題 (2026-11-07)。HNFPRE の記録 (analysis/hnfpre_20261107.md §2) の「合同式が出る」問題。"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "cbcimpl_20261029"))
# 合同式が上下限を詰めない (分枝・伝播・カットにだけ使う) 問題
CONG = ["brazil3", "mzzv11", "neos-3656078-kumeu", "piperout-08", "piperout-27", "radiationm18-12-05", "radiationm40-10-02", "rocI-4-11",
        "rococoB10-011000", "rococoC10-001000", "snp-02-004-104", "traininstance2", "traininstance6", "lectsched-5-obj", "8div-n59k10", "flugpl"]
# 2 回目 (collection の探索 scan.py で見つけたもの)
HITS2 = ["rococoB10-011000", "rococoC10-001000", "flugpl", "ns1631475"]
SETS = {"cong": CONG, "hits2": HITS2}
try:
    import probs as _p58
    SETS["p58"] = list(_p58.PROBS) if hasattr(_p58, "PROBS") else list(_p58.probs)
except Exception:
    pass
