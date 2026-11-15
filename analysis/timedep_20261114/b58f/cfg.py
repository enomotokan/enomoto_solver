"""timedep の 58 問の設定 (6 回目、4 スレッド・60 秒、種 1〜4)。すべて束 cand1114 の上に `ENOMOTO_MIP_TIMEDEP` の文字を足す (文字の意味は src/mip/mod.rs の `timedep`、
記録は analysis/timedep_20261114.md の 7 節)。
6 回目: 5 回目 (b58e) の組 C から、nw04・dcmulti を遅くした c を外し、根のサブ MIP を LP の反復で止める u (係数 5) を足した組 D を base と対で確かめる。
1 文字ずつ外した形 (b58e の loo) で 1 問ずつの害が出た t (binkar10_1)・n (neos17) と、新しい u を外した形も同じ回で並べる。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[2] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
LETTERS = "mplwnNebtyu"
def td(v): return {**CAND1114, "ENOMOTO_MIP_TIMEDEP": v, "ENOMOTO_T_MIP_TIMEDEP_SEP_COST": "5", "ENOMOTO_T_MIP_TIMEDEP_LNS_MIN": "10"}
CFGS = {
    "base": dict(CAND1114),
    "D": td(LETTERS),
    "D-t": td(LETTERS.replace("t", "")),
    "D-n": td(LETTERS.replace("n", "").replace("N", "")),
    "D-u": td(LETTERS.replace("u", "")),
}
LOO = {f"D-{x}": td(LETTERS.replace(x, "").replace("N", "") if x == "n" else LETTERS.replace(x, "")) for x in LETTERS}
CFGS_ALL = {**LOO, **CFGS}
