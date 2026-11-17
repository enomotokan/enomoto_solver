"""手間の単位 (`ENOMOTO_MIP_TICKS`) の確かめ、2 回目 (analysis/timedep_20261114.md の 11 節、4 スレッド・60 秒、58 問 × 種 1〜4)。
base = cand1116f、K2 = base + `ENOMOTO_MIP_TICKS=ntwumcfl` (k58 の ntwu に、MPSEP の移動・CUTSEL_PROBE・前処理の早い FJ/Local-MIP・局所探索を足した形)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CFGS = {"base": dict(_m.CAND1116F), "K2": {**_m.CAND1116F, "ENOMOTO_MIP_TICKS": "ntwumcfl"}}
CFGS_ALL = CFGS
