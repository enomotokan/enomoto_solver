"""TICKS の `m` (MPSEP の最適面の移動の止め方) の手当ての確かめ (analysis/timedep_20261114.md の 12.11 節)。基準は c17_mr8
(cand1117 + ENOMOTO_T_MIP_CUT_MIN_ROUNDS=8、12.10 節)。
- c17_mr8: 入れ替え 16 回で止める (今の `m`)
- c17_mr8_p8・c17_mr8_p4: + ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0=8・4
- c17_mr8_mtime: TICKS から `m` を外す (時間版: 1 回の移動を 0.25 x 根の LP の秒 (0.01〜0.1 秒) の壁時計で止める)
- c17_mr8_mtk: + ENOMOTO_T_MIP_TICKS_MPSEP_MOVE_FRAC=0.25 (時間版と同じ式の手間の期限 clamp(0.25 x 根の LP の手間, 1e7, 1e8))"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1117", Path(__file__).resolve().parents[2] / "cand1117_20261116" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
B = {**_m.CAND1117, "ENOMOTO_T_MIP_CUT_MIN_ROUNDS": "8"}
assert B["ENOMOTO_MIP_TICKS"] == "ntwumcflshk"
CFGS = {"c17_mr8": B,
        "c17_mr8_p8": {**B, "ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0": "8"},
        "c17_mr8_p4": {**B, "ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0": "4"},
        "c17_mr8_mtime": {**B, "ENOMOTO_MIP_TICKS": "ntwucflshk"},
        "c17_mr8_mtk": {**B, "ENOMOTO_T_MIP_TICKS_MPSEP_MOVE_FRAC": "0.25"}}
CFGS_ALL = CFGS
