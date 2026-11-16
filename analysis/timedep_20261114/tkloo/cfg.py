"""TICKS の文字ごとの切り分け (analysis/timedep_20261114.md の 12.8 節): cand1117 で neos-827175・10teams が遅い原因の文字を探す。
- cand1117 (analysis/cand1117_20261116/cfg.py の CAND1117: cand1116 + TICKS=ntwumcflshk + DETHASH + DFSBB=2)
- c17_noticks: cand1117 から ENOMOTO_MIP_TICKS だけ外す (DETHASH・DFSBB は残す)
- c17_m<文字>: cand1117 の TICKS から 1 文字だけ外す (11 通り)"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1117", Path(__file__).resolve().parents[2] / "cand1117_20261116" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
C17 = dict(_m.CAND1117)
LET = C17["ENOMOTO_MIP_TICKS"]
assert LET == "ntwumcflshk", LET
CFGS = {"cand1117": C17, "c17_noticks": {k: v for k, v in C17.items() if k != "ENOMOTO_MIP_TICKS"}}
for c in LET:
    CFGS[f"c17_m{c}"] = {**C17, "ENOMOTO_MIP_TICKS": LET.replace(c, "")}
CFGS_ALL = CFGS
