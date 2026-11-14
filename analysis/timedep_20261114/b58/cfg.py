"""timedep の 58 問の設定 (4 スレッド・60 秒)。すべて束 cand1114 の上に `ENOMOTO_MIP_TIMEDEP` の文字を足す (文字の意味は src/mip/mod.rs の `timedep`、
記録は analysis/timedep_20261114.md)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[2] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
def td(v): return {**CAND1114, "ENOMOTO_MIP_TIMEDEP": v}
CFGS = {
    "base": dict(CAND1114),
    "td1": td("mpl"),        # (1) C9・C2・D5
    "td1x": td("mplwneb"),   # (1) + C1・B1・B7・B19 (1 スレッドの A/A で次に分かれた所)
    "td2": td("r"),          # (2) E1
    "td3": td("cb"),         # (3) C7・B19
    "tdall": td("mplwnercb"),
}
