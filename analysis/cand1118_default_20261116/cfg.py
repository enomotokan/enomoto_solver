"""cand1118 を既定にした版の確かめ (2026-11-16)。def = 環境変数なし (新しい既定)、c1118 = cand1118 の環境変数を明示、
rev = 既定にした 12 個を 0.1.7 までの動きに戻す (0・off・数値は元の値)。def と c1118 は同じ動き、rev は cand1114 (0.1.7 の既定) と同じ動きになるはず。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

CAND1118 = _load("cand1118_20261116/cfg.py", "cfg1118").CAND1118
REV = {"ENOMOTO_MIP_OBJPROBE": "0", "ENOMOTO_MIP_POOL_TAB": "0", "ENOMOTO_MIP_RHEUR": "0", "ENOMOTO_MIP_ROBUST": "0",
       "ENOMOTO_MIP_SUBMIP_HALT": "0", "ENOMOTO_MIP_TC": "0", "ENOMOTO_MIP_TICKS": "0", "ENOMOTO_MIP_DETHASH": "0",
       "ENOMOTO_MIP_DFSBB": "0", "ENOMOTO_MIP_TREE2_INTAGG": "0", "ENOMOTO_T_MIP_CUT_MIN_ROUNDS": "0", "ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0": "16"}
CFGS = {"def": {}, "c1118": CAND1118, "rev": REV, "def_t1": {}, "c1118_t1": CAND1118}  # _t1 は 1 スレッド (run.py)
