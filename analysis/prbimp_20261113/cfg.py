"""PRBIMP の設定。cand1114 (analysis/cand1114_20261113/cfg.py) の上に足す。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
CAND1114 = _m.CAND1114
CFGS = {
    "def": {},
    "c14": dict(CAND1114),
    "c14clq": dict(CAND1114, ENOMOTO_MIP_CLQPRB="1d"),
}
CFGS["c14clqn"] = dict(CAND1114, ENOMOTO_MIP_CLQPRB="1dn")
CFGS["c14clqg"] = dict(CAND1114, ENOMOTO_MIP_CLQPRB="1dg")
