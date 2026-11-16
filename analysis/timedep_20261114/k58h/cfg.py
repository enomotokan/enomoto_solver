"""手間の単位 (`ENOMOTO_MIP_TICKS`) の r・j の確かめ (analysis/timedep_20261114.md の 12.5 節、4 スレッド・60 秒、58 問 × 種 1〜4)。
base = cand1116 + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` (次の束の形)、KRJ = base の TICKS を `ntwumcflshkrj` にしたもの。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
_B = {**_m.CAND1116, "ENOMOTO_MIP_DETHASH": "1"}
CFGS = {"base": {**_B, "ENOMOTO_MIP_TICKS": "ntwumcflshk"}, "KRJ": {**_B, "ENOMOTO_MIP_TICKS": "ntwumcflshkrj"}}
CFGS_ALL = CFGS
