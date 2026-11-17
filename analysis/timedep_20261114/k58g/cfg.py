"""手間の単位 (`ENOMOTO_MIP_TICKS`) の s・h・k の確かめ、種 5〜8 (analysis/timedep_20261114.md の 12.3 節、4 スレッド・60 秒、58 問 × 種 5〜8)。
base = cand1116 (深さ優先寄りなし)、K3DS = base + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` (k58f と同じ)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CFGS = {"base": dict(_m.CAND1116), "K3DS": {**_m.CAND1116, "ENOMOTO_MIP_DETHASH": "1", "ENOMOTO_MIP_TICKS": "ntwumcflshk"}}
CFGS_ALL = CFGS
