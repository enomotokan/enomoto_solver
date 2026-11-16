"""b58u の 8 つの設定 (analysis/treeclose_20261114.md 9.41 節、利用者の指示: 下界最小のノードを取る頻度の最適化)。b58r で HiGHS 型 (hgs) が勝ったので、
cand1116 + `ENOMOTO_MIP_DFSBB=2` の上で、待ち行列から取る k 回に 1 回下界最小の k (`ENOMOTO_T_MIP_HGS_BB_EVERY`) = 1・3・10・30 と、
1 回の潜りのノード数の上限 (`ENOMOTO_T_MIP_HGS_PLUNGE_NODES`) = 100 (HiGHS と同じ)・300 を掃引する。k10_p100 が b58r の hgs と同じ。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
CFGS = {f"k{k}_p{pn}": dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="2", ENOMOTO_T_MIP_HGS_BB_EVERY=str(k), ENOMOTO_T_MIP_HGS_PLUNGE_NODES=str(pn))
        for pn in (100, 300) for k in (1, 3, 10, 30)}
