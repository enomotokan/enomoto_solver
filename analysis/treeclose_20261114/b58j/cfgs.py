"""b58j の 2 つの設定 (analysis/treeclose_20261114.md 9.19 節)。どちらも束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116) の上。
nb1 = 試作 `ENOMOTO_MIP_NODESEL_BANDIT=1` (割引付き UCB、腕はほぼ交互になる)。
nb4 = `=4` (学習なし、区間ごとに既定と深さ優先寄り (d3 + TC=k) を決まった並びで交互に: 割合 `ENOMOTO_T_MIP_NB4_SHARE` 0.5、区間 `ENOMOTO_T_MIP_NB_WORK` 2 万反復)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
CFGS = {"nb1": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="1"), "nb4": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="4")}
