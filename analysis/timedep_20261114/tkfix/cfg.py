"""cand1117 の neos-827175 の遅れの手当ての確かめ (analysis/timedep_20261114.md の 12.10 節)。
- cand1117 (analysis/cand1117_20261116/cfg.py の CAND1117)
- c17_mr8: + ENOMOTO_T_MIP_CUT_MIN_ROUNDS=8 (外側の根のカットのループを 8 ラウンドまでは時間の上限で止めない。既定 0 の試作の鍵)
- c17_w: + 当てはめ直した手間の重み (12.9 節、eff7eac07: ENOMOTO_T_TICKS_W_<部分>)
- c17_mr8w: 両方"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1117", Path(__file__).resolve().parents[2] / "cand1117_20261116" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
C17 = dict(_m.CAND1117)
MR8 = {"ENOMOTO_T_MIP_CUT_MIN_ROUNDS": "8"}
W = {f"ENOMOTO_T_TICKS_W_{k.upper()}": str(v) for k, v in
     dict(lp=4.629, prop=5.807, sep=31.378, pre=93.979, heur=32.318, node=136.188, lpit=10598.311, lu=2.132, lum=5.345).items()}
CFGS = {"cand1117": C17, "c17_mr8": {**C17, **MR8}, "c17_w": {**C17, **W}, "c17_mr8w": {**C17, **MR8, **W}}
CFGS_ALL = CFGS
