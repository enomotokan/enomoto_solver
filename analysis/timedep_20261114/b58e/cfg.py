"""timedep の 58 問の設定 (5 回目、4 スレッド・60 秒、種 1〜4)。すべて束 cand1114 の上に `ENOMOTO_MIP_TIMEDEP` の文字を足す (文字の意味は src/mip/mod.rs の `timedep`、
記録は analysis/timedep_20261114.md の 6・7 節)。
5 回目: 利用者の方針「時間依存の部分をなるべく減らす。損得のないぐらいなら上出来」により、損の出ない値でできるだけ多くの文字を入れた組 C を base と対で確かめる。
C = mplwnNebtyc (12 文字のうち r 以外) + 分離の重さ SEP_COST 5 (t、mas76 の害の元) + LNS の最小 10 x 根 (n)。Cr = C + r (根の LP の競争)。
揃って遅い問題が出たときのため、C から 1 文字ずつ外した形 LOO (C-m など) も用意する (shards_loo.json、必要なときだけ回す)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[2] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
LETTERS = "mplwnNebtyc"
def td(v): return {**CAND1114, "ENOMOTO_MIP_TIMEDEP": v, "ENOMOTO_T_MIP_TIMEDEP_SEP_COST": "5", "ENOMOTO_T_MIP_TIMEDEP_LNS_MIN": "10"}
CFGS = {
    "base": dict(CAND1114),
    "C": td(LETTERS),
    "Cr": td(LETTERS + "r"),
}
# 1 文字ずつ外した形 (N は n に従うので n を外すときは N も外す)
LOO = {f"C-{x}": td(LETTERS.replace(x, "").replace("N", "") if x == "n" else LETTERS.replace(x, "")) for x in LETTERS}
CFGS_ALL = {**CFGS, **LOO}
