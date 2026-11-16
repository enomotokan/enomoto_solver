"""b58i の 2 つの設定 (analysis/treeclose_20261114.md 9.16 節、利用者の案)。どちらも束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116) の上。
d3k = 深さ優先寄り d3 (`PLUNGE_QUOT=1`・`SIBLING_QUOT=1`、潜りのノード数の上限は既定) + `TC=k` (`ENOMOTO_MIP_TC=bsk`)。
d3kq = d3k + `TC=q` (潜りの途中でも主のスレッドの LP の反復 5000 ごとに、伝播のプロービング 50 列・カットプールの伝播・分離とヒューリスティクス)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
D3K = dict(BASE, ENOMOTO_MIP_TC="bsk", ENOMOTO_T_MIP_PLUNGE_QUOT="1", ENOMOTO_T_MIP_SIBLING_QUOT="1")
CFGS = {"d3k": D3K, "d3kq": dict(D3K, ENOMOTO_MIP_TC="bskq", ENOMOTO_T_MIP_TC_Q_WORK="5000", ENOMOTO_T_MIP_TC_Q_PROBES="50")}
