"""b58q の 3 つの設定 (analysis/treeclose_20261114.md 9.33 節、利用者の指示: =5 の学習が効いているかの対照実験)。
d2q = b58o・b58p の d2q (=5 EI、PQ・SQ、区間 2000・窓 150・REFIT 10・NOINC・報酬 q10)。d2n = d2q + `NB5_NOGP=1` (GP・獲得関数を使わず、
初期点の後は Halton の候補を決まった順に試すだけ = 学習なしで空間を均等に試す)。nb1 = `=1` (同じ機械・同じ日で並べて日の揺れを除くため)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
CFGS = {
    "d2q": dict(BASE, ENOMOTO_MIP_NB5_DIMS="ps", ENOMOTO_MIP_NB5_REWARD="q10"),
    "d2n": dict(BASE, ENOMOTO_MIP_NB5_DIMS="ps", ENOMOTO_MIP_NB5_REWARD="q10", ENOMOTO_MIP_NB5_NOGP="1"),
    "nb1": dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="1"),
}
