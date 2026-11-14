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
CFGS["c14clqr"] = dict(CAND1114, ENOMOTO_MIP_CLQPRB="1dgr")  # g + r: 含意を使う形が既定の形より多く得なければ既定の結果に戻す
CFGS["c14clqh"] = dict(CAND1114, ENOMOTO_MIP_CLQPRB="1dgrh")  # g + r + h: 含意を使う形にしたら、既定の形の問題で locks の sub-MIP を解いた解を最初の暫定解の候補にする
