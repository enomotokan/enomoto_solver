"""cand1113 の leave-one-out (2026-11-12)。58 問 60 秒で 4 種そろって遅い neos-960392・binkar10_1・roll3000・pk1 と、
速い側の対照 10teams・neos8・h80x6320d で、どのまとまりが遅さ・速さの元かを探す。
"-X" は CAND1113 からまとまり X (../cfg.py のコメントの単位) を外した形。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1113", Path(__file__).resolve().parents[1] / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1113 = _m.CAND1113

GROUPS = {
    "ewk": ["ENOMOTO_MIP_ERWAIT"],
    "tenc": ["ENOMOTO_MIP_TREEEFF", "ENOMOTO_MIP_NODECNT"],
    "utw": ["ENOMOTO_MIP_IPMRC"],
    "n05": ["ENOMOTO_MIP_RSTCRIT"],
    "net": ["ENOMOTO_MIP_FOLD"],
    "LMBOOST": ["ENOMOTO_MIP_LMBOOST"],
    "prepar": ["ENOMOTO_MIP_PREPAR"],
    "ncm": ["ENOMOTO_MIP_NODE_CUT_MAXLEN"],
    "ns250s": ["ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP", "ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL"],
    "prune1112": ["ENOMOTO_MIP_SUBFIX", "ENOMOTO_MIP_HNFPRE", "ENOMOTO_MIP_NO_ORBITOPE_TWO_COL",
                  "ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE", "ENOMOTO_MIP_LP_PROBE_FINISH"],
    "DUALEQ": ["ENOMOTO_MIP_DUALEQ"],
    "ROOTSTOP": ["ENOMOTO_MIP_ROOTSTOP"],
    "RCSTRONG": ["ENOMOTO_MIP_RCSTRONG"],
    "LMIP_SWAP": ["ENOMOTO_MIP_LMIP_SWAP"],
    "SYMPRE": ["ENOMOTO_MIP_SYMPRE"],
    "PLSB": ["ENOMOTO_MIP_PLSB"],
    "GAP1": ["ENOMOTO_MIP_GAP1"],
    "GAP4": ["ENOMOTO_MIP_GAP4"],
    "OBJINT": ["ENOMOTO_MIP_OBJINT"],
    "CLQPRB": ["ENOMOTO_MIP_CLQPRB"],
}
assert sorted(k for v in GROUPS.values() for k in v) == sorted(CAND1113)
CFGS = {"def": {}, "cand1113": CAND1113}
for _g, _ks in GROUPS.items():
    CFGS["-" + _g] = {k: v for k, v in CAND1113.items() if k not in _ks}
