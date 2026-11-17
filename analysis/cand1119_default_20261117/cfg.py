"""cand1119 を既定にした版の確かめ (2026-11-17)。def = 環境変数なし (新しい既定)、c1119 = cand1119 の環境変数を明示、
rev = 既定にした 3 つ (+係数) を cand1118 の動きに戻す。def と c1119 は同じ動き、rev は cand1118 (前の既定) と同じ動きになるはず。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

CAND1119 = _load("cand1119_20261117/cfg.py", "cfg1119").CAND1119
REV = {"ENOMOTO_MIP_CLIQUE_CUTS": "fallback", "ENOMOTO_MIP_SYM_TWINS": "0", "ENOMOTO_MIP_CUTSEL_PROBE_AFFORD": "0"}
CFGS = {"def": {}, "c1119": CAND1119, "rev": REV, "def_t1": {}, "c1119_t1": CAND1119}  # _t1 は 1 スレッド (run.py)
