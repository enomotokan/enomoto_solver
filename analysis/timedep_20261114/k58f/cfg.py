"""手間の単位 (`ENOMOTO_MIP_TICKS`) の s・h・k の確かめ (analysis/timedep_20261114.md の 12.1 節、4 スレッド・60 秒、58 問 × 種 1〜4)。
base = cand1116 (深さ優先寄りなし)、K3D = base + `ENOMOTO_MIP_TICKS=ntwumcfl` + `ENOMOTO_MIP_DETHASH=1`、
K3DS = base + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1`。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
_D = {**_m.CAND1116, "ENOMOTO_MIP_DETHASH": "1"}
CFGS = {"base": dict(_m.CAND1116), "K3D": {**_D, "ENOMOTO_MIP_TICKS": "ntwumcfl"}, "K3DS": {**_D, "ENOMOTO_MIP_TICKS": "ntwumcflshk"}}
CFGS_ALL = CFGS
