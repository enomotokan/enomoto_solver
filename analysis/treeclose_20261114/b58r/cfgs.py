"""b58r の 5 つの設定 (analysis/treeclose_20261114.md 9.36 節、利用者の案: 基本は深さ優先で、たまに下界最小のノードを取る)。どれも束 cand1116 の上。
w1・w5・w25 = `ENOMOTO_MIP_DFSBB=1` で W = 1・5・25 × 根の LP の反復 (下限 1000)。nb1 = `=1`、d2n = =5 の学習なし (b58q の d2n)。
追加: hgs = HiGHS と同じ木の進め方 (`ENOMOTO_MIP_DFSBB=2`)、hgsw = その手間版 (`=3`、W = 5 倍)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
CFGS = {
    "w1": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="1", ENOMOTO_T_MIP_DFSBB_W="1"),
    "w5": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="1", ENOMOTO_T_MIP_DFSBB_W="5"),
    "w25": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="1", ENOMOTO_T_MIP_DFSBB_W="25"),
    "nb1": dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="1"),
    "d2n": dict(BASE, ENOMOTO_MIP_NB5_DIMS="ps", ENOMOTO_MIP_NB5_REWARD="q10", ENOMOTO_MIP_NB5_NOGP="1"),
    # 追加 (利用者の指示): HiGHS と同じ木の進め方 (潜りは深さ優先・1 回の潜り 100 ノード・待ち行列は 10 回に 1 回下界最小、ほかは 0.5 下界 + 0.5 推定値)、
    # hgsw はノード数の上限を手間の上限 W = 5 × 根の LP の反復に置き換えたもの。どちらも ENOMOTO_MIP_DFSBB=2・3 のある版 (9.37 節のコミット以降) が要る
    "hgs": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="2"),
    "hgsw": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="3", ENOMOTO_T_MIP_DFSBB_W="5"),
}
