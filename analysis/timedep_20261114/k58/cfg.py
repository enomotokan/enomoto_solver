"""手間の単位 (`ENOMOTO_MIP_TICKS`) の確かめ (analysis/timedep_20261114.md の 11 節、4 スレッド・60 秒、58 問 × 種 1〜4)。
base = cand1116f (analysis/cand1116_20261115/cfg.py の CAND1116F、HALT 入り)、K = base + `ENOMOTO_MIP_TICKS=ntwu`。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CFGS = {"base": dict(_m.CAND1116F), "K": {**_m.CAND1116F, "ENOMOTO_MIP_TICKS": "ntwu"}}
CFGS_ALL = CFGS
