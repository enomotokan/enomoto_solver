"""手間の単位 (`ENOMOTO_MIP_TICKS`) の確かめ、3 回目 (analysis/timedep_20261114.md の 11.7 節、4 スレッド・60 秒、58 問 × 種 1〜4)。
base = cand1116f、K3 = base + `ENOMOTO_MIP_TICKS=ntwumcfl` (m を入れ替えの数の上限に直した版)、K3D = K3 + `ENOMOTO_MIP_DETHASH=1` (12 節)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
K3 = {**_m.CAND1116F, "ENOMOTO_MIP_TICKS": "ntwumcfl"}
CFGS = {"base": dict(_m.CAND1116F), "K3": K3, "K3D": {**K3, "ENOMOTO_MIP_DETHASH": "1"}}
CFGS_ALL = CFGS
