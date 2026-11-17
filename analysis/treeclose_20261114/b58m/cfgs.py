"""b58l の 4 つの設定 (analysis/treeclose_20261114.md 9.21 節)。どれも束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116) の上。
nb1 = `ENOMOTO_MIP_NODESEL_BANDIT=1`、nb4 = `=4` (学習なしの交互)、nb5u = `=5` (解きながらのベイズ最適化、獲得は UCB)、nb5e = `=5` で獲得は EI。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
CFGS = {
    "nb1": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="1"),
    "nb4": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="4"),
    "nb5u": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="5"),
    "nb5e": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei"),
}
