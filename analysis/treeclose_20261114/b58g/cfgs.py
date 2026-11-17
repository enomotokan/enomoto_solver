"""b58g の 2 つの設定 (analysis/treeclose_20261114.md 9.12 節)。base = 束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116)。
nb = base + 試作 `ENOMOTO_MIP_NODESEL_BANDIT` (潜り方を区間ごとに既定と深さ優先寄り (d3 + TC=k) の 2 本の腕から選ぶ、主のスレッドの木だけ)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
CFGS = {"base": BASE, "nb": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="1")}
