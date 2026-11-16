"""b58p の 3 つの設定 (analysis/treeclose_20261114.md 9.32 節): b58o (9.31 節) の d2q・d2g・d3q を、GP 最良点を予測の平均の最大に直した版
(dbf0164b4 以降) で取り直す。選ぶ点は b58o と同じ仕組み (違いはログの最良点の意味だけ) なので、成績は b58o と比べられる。
区間 2000・窓 150・REFIT 10・割引は反復 2 万ごと・`=5` EI・`NB5_NOINC=1`。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
CFGS = {
    "d2q": dict(BASE, ENOMOTO_MIP_NB5_DIMS="ps", ENOMOTO_MIP_NB5_REWARD="q10"),
    "d2g": dict(BASE, ENOMOTO_MIP_NB5_DIMS="ps", ENOMOTO_MIP_NB5_REWARD="gap"),
    "d3q": dict(BASE, ENOMOTO_MIP_NB5_DIMS="psu", ENOMOTO_MIP_NB5_REWARD="q10"),
}
