"""timedep の 58 問の設定 (7 回目、4 スレッド・60 秒、種 1〜4)。すべて束 cand1114 の上に `ENOMOTO_MIP_TIMEDEP` の文字を足す (記録は analysis/timedep_20261114.md の 7 節)。
6 回目 (b58f) で損のなかった E (= D-t = mplwnNebyu) は、1 スレッドの決定性が 33/58 (t を含む C + u は 53/58) と低い。
t (木の分離の予算を手間で数える) の係数を変えた形を E に足して、損のない値があるかを確かめる。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[2] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
def td(v, **kw): return {**CAND1114, "ENOMOTO_MIP_TIMEDEP": v, "ENOMOTO_T_MIP_TIMEDEP_LNS_MIN": "10", **kw}
CFGS = {
    "base": dict(CAND1114),
    "E": td("mplwnNebyu"),
    "Et2": td("mplwnNebytu", ENOMOTO_T_MIP_TIMEDEP_SEP_COST="2"),
    "Et5f": td("mplwnNebytu", ENOMOTO_T_MIP_TIMEDEP_SEP_COST="5", ENOMOTO_T_MIP_TREE_SEP_TIME_FRAC="0.2"),
}
CFGS_ALL = CFGS
