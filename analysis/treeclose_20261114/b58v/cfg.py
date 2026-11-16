"""b58v の 3 つの設定 (analysis/treeclose_20261114.md 9.43 節、HiGHS 型の木の進め方を束に入れるかの判断)。土台は timedep で採用した
cand1116 + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` (k58g の K3DS と同じ。6e875bdf7 以降の DETHASH の直しを含む版で回す)。
base = 土台、hgs = 土台 + `ENOMOTO_MIP_DFSBB=2` (HiGHS 型、10 回に 1 回下界最小・潜り 100 ノード、b58r の最良)、
h30 = 土台 + `DFSBB=2`・`HGS_BB_EVERY=30`・`HGS_PLUNGE_NODES=300` (b58u の最良)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = {**_m.CAND1116, "ENOMOTO_MIP_DETHASH": "1", "ENOMOTO_MIP_TICKS": "ntwumcflshk"}
CFGS = {
    "base": BASE,
    "hgs": {**BASE, "ENOMOTO_MIP_DFSBB": "2"},
    "h30": {**BASE, "ENOMOTO_MIP_DFSBB": "2", "ENOMOTO_T_MIP_HGS_BB_EVERY": "30", "ENOMOTO_T_MIP_HGS_PLUNGE_NODES": "300"},
}
CFGS_ALL = CFGS
