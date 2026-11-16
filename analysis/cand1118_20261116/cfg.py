"""束 cand1118 (2026-11-16、案)。cand1117 (analysis/cand1117_20261116/cfg.py: cand1116 + TICKS=ntwumcflshk + DETHASH=1 + DFSBB=2) に、
  - ENOMOTO_T_MIP_CUT_MIN_ROUNDS=8
  - ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0=8
を足したもの (利用者の判断、treeclose 9.49 節の後)。DFSBB=2 (k10・p100) はそのまま (supportcase40 は既知の例外)。
比べる相手は cand1117 と前の基準 cand1114、Mittelmann では HiGHS 1.15.1 も。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

_c17 = _load("cand1117_20261116/cfg.py", "cfg1117")
CAND1114 = _c17.CAND1114
CAND1117 = _c17.CAND1117
CAND1118 = {**CAND1117, "ENOMOTO_T_MIP_CUT_MIN_ROUNDS": "8", "ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0": "8"}
CFGS = {"cand1114": CAND1114, "cand1117": CAND1117, "cand1118": CAND1118, "highs": {}}
