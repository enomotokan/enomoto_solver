"""cand1115 (TC=b) の 1 つ外し (loo)。58 問の P で遅れぎみだった問題 (mc11・neos17・neos-911970・swath1・roll3000・p200x1188c・physiciansched6-2)
で、どの項目が遅らせているかを見る。設定名 = 外した項目。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1115", Path(__file__).resolve().parent / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114, CAND1115 = _m.CAND1114, _m.CAND1115
GROUPS = {
    "objprobe": ["ENOMOTO_MIP_OBJPROBE"],
    "intagg": ["ENOMOTO_MIP_TREE2_INTAGG"],
    "rmc150": ["ENOMOTO_T_MIP_ROOT_MAX_CUTS"],
    "pooltab": ["ENOMOTO_MIP_POOL_TAB"],
    "timedep": ["ENOMOTO_MIP_TIMEDEP", "ENOMOTO_T_MIP_TIMEDEP_SEP_COST", "ENOMOTO_T_MIP_TIMEDEP_LNS_MIN"],
    "robust": ["ENOMOTO_MIP_ROBUST"],
    "rheur": ["ENOMOTO_MIP_RHEUR"],
    "tc": ["ENOMOTO_MIP_TC"],
}
CFGS = {"cand1115": CAND1115, **{f"-{g}": {k: v for k, v in CAND1115.items() if k not in ks} for g, ks in GROUPS.items()}}
PROBS = ["mc11", "neos17", "neos-911970", "swath1", "roll3000", "p200x1188c", "physiciansched6-2"]
