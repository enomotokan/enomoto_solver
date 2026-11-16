"""b58t の 3 つの設定 (analysis/treeclose_20261114.md 9.39 節、利用者の案: 背景の FJ・Local-MIP を有望な部分問題に差し替える。親の指示で効果を分けた)。
base = 束 cand1116 + `ENOMOTO_MIP_DFSBB=4` (d = leaf、c = 3。b58s の結果で決め直す、今は仮。木では背景のスレッドなし)、
fjg = base + `ENOMOTO_MIP_FJSUB=1` + `ENOMOTO_T_MIP_FJSUB_RHO=0` (候補を置かない = 木の間も全体の問題で FJ・Local-MIP を回し続けるだけ)、
fjs = base + `FJSUB=1` + `RHO=1.0` (分数値の整数列が根以下のノードを候補に。下界の条件はそのまま)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
WIN = dict(ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_MIP_NB5_NOINC="1", **WIN)
BASE = dict(_m.CAND1116, ENOMOTO_MIP_DFSBB="4", ENOMOTO_MIP_DFSBB_D="leaf", ENOMOTO_T_MIP_DFSBB_C="3")
CFGS = {
    "base": BASE,
    "fjg": dict(BASE, ENOMOTO_MIP_FJSUB="1", ENOMOTO_T_MIP_FJSUB_RHO="0"),
    "fjs": dict(BASE, ENOMOTO_MIP_FJSUB="1", ENOMOTO_T_MIP_FJSUB_RHO="1.0"),
}
