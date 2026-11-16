"""束 cand1117 (2026-11-16、案)。cand1116 (analysis/cand1116_20261115/cfg.py) に、
  - ENOMOTO_MIP_TICKS=ntwumcflshk + ENOMOTO_MIP_DETHASH=1 (timedep で採用済み、analysis/timedep_20261114.md 12 節。6e875bdf7 以降の DETHASH の直しを含む版)
  - ENOMOTO_MIP_DFSBB=2 (HiGHS と同じ木の進め方: 潜りは深さ優先で 100 ノードで打ち切り、待ち行列は 10 回に 1 回下界最小、ほかは 0.5 下界 + 0.5 推定値。
    analysis/treeclose_20261114.md 9.40・9.44 節。b58v の 58 問 × 種 1〜4 で −5.3% [−9.6, −1.3]・解けた 178/175・食い違い 0)
を足したもの。c19fc41d9 以降の版で回す。比べる相手は cand1116 と前の基準 cand1114、Mittelmann では HiGHS 1.15.1 も。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

_c16 = _load("cand1116_20261115/cfg.py", "cfg1116")
CAND1114 = _c16.CAND1114
CAND1116 = _c16.CAND1116
CAND1117 = {**CAND1116, "ENOMOTO_MIP_TICKS": "ntwumcflshk", "ENOMOTO_MIP_DETHASH": "1", "ENOMOTO_MIP_DFSBB": "2"}
CFGS = {"cand1114": CAND1114, "cand1116": CAND1116, "cand1117": CAND1117, "highs": {}}
