"""b58o の 3 つの設定 (analysis/treeclose_20261114.md 9.26 節、利用者の指示: =5 の次元を減らす)。
どれも束 cand1116 の上の `=5` EI + `NB5_NOINC=1` (暫定解がない区間の観測は捨て、点も選び直さない)。
d3 = 3 次元 (`NB5_DIMS=psu`)、d2 = PLUNGE_QUOT・SIBLING_QUOT (u は 1/7 に固定)、d1 = SIBLING_QUOT だけ (PLUNGE_QUOT 0.25・u 1/7 に固定)。
区間の長さ・窓・REFIT (WIN) は b58n の結果で一番良かったものに合わせる (TODO: b58n の集計の後に決める。今は b58l と同じ区間 20000・窓 40)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="20000", ENOMOTO_T_MIP_NB5_WINDOW="40", ENOMOTO_T_MIP_NB5_REFIT="1", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
CFGS = {
    "d3": dict(BASE, ENOMOTO_MIP_NB5_DIMS="psu"),
    "d2": dict(BASE, ENOMOTO_MIP_NB5_DIMS="ps"),
    "d1": dict(BASE, ENOMOTO_MIP_NB5_DIMS="s"),
}
