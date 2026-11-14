"""timedep の 58 問の設定 (2 回目、4 スレッド・60 秒)。すべて束 cand1114 の上に `ENOMOTO_MIP_TIMEDEP` の文字を足す (文字の意味は src/mip/mod.rs の `timedep`、
記録は analysis/timedep_20261114.md)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[2] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
def td(v): return {**CAND1114, "ENOMOTO_MIP_TIMEDEP": v}
# 2 回目 (1 回目 b58 の結果を受けて文字ごとに切り分ける。m は上限を直した版 (入れ替え 16 回))
CFGS = {
    "base": dict(CAND1114),
    "m": td("m"),
    "pl": td("pl"),
    "c": td("c"),
    "b": td("b"),
    "w": td("w"),
    "nNe": td("nNe"),
    "ty": td("ty"),
    "mplcb": td("mplcb"),
}
