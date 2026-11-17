"""b58s の 6 つの設定 (analysis/treeclose_20261114.md 9.38 節、利用者の案: たまに下界最小を取るタイミングを、手間ではなく整数変数の数と分枝の回数で決める)。
どれも束 cand1116 の上。f03・f1・l1・l3 = `ENOMOTO_MIP_DFSBB=4` (潜りは dfsbb と同じ、1 回の潜りのノード数が L = max(10, round(c × d)) を越えたら打ち切って下界最小を取る)。
f03・f1 = d が根の LP 解で分数値の整数変数の数 (`ENOMOTO_MIP_DFSBB_D=frac`)、c = 0.3・1。l1・l3 = d が葉・刈り込みで終わった潜りの深さの平均 (`leaf`)、c = 1・3
(親の指示で変えた。frac の d は葉の深さと桁が合わない: mcsched 1263 対 28)。
対照 2 つ (CTRL1・CTRL2) は b58r の集計の後に「b58r でいちばん良い設定」と nb1 に差し替える。b58r の集計 (9.40 節) で hgs に決めた (dfsbb、W = 5 倍) と nb1。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
CTRL1 = ("hgs", dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="2"))  # b58r でいちばん良かった HiGHS 型 (9.40 節)
CTRL2 = ("nb1", dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="1"))
CFGS = {
    "f03": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="4", ENOMOTO_MIP_DFSBB_D="frac", ENOMOTO_T_MIP_DFSBB_C="0.3"),
    "f1": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="4", ENOMOTO_MIP_DFSBB_D="frac", ENOMOTO_T_MIP_DFSBB_C="1"),
    "l1": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="4", ENOMOTO_MIP_DFSBB_D="leaf", ENOMOTO_T_MIP_DFSBB_C="1"),
    "l3": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="4", ENOMOTO_MIP_DFSBB_D="leaf", ENOMOTO_T_MIP_DFSBB_C="3"),
    CTRL1[0]: CTRL1[1],
    CTRL2[0]: CTRL2[1],
}
