"""timedep の 58 問の設定 (4 回目、4 スレッド・60 秒、種 1〜4)。すべて束 cand1114 の上に `ENOMOTO_MIP_TIMEDEP` の文字を足す (文字の意味は src/mip/mod.rs の `timedep`、
記録は analysis/timedep_20261114.md)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[2] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
def td(v): return {**CAND1114, "ENOMOTO_MIP_TIMEDEP": v}
# 4 回目: 利用者の判断で候補にした p・l・b・m (m は入れ替え 16 回の版、c は 7e192cae で直した版が入る版で回す) を 58 問 × 種 1〜4 で確かめる
CFGS = {
    "base": dict(CAND1114),
    "pl": td("pl"),
    "b": td("b"),
    "m": td("m"),
    "plbm": td("plbm"),
}
