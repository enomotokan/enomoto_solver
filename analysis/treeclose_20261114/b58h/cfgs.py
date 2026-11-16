"""b58h の 2 つの設定 (analysis/treeclose_20261114.md 9.15 節)。どちらも束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116) の上。
nb1 = 試作 `ENOMOTO_MIP_NODESEL_BANDIT=1` (腕を報酬そのまま・UCB の係数 0.5 で選ぶ。腕はほぼ交互になる)。
nb2 = `=2` (報酬を |報酬| の指数移動平均で割って正規化し、UCB の係数 0.2。問題ごとに腕を寄せる調整版)。
nb3 = `=3` (正規化した報酬の差で腕 1 の目標の割合を決め、どちらの腕も最低 2 割使う)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
CFGS = {"nb1": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="1"), "nb2": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="2"), "nb3": dict(BASE, ENOMOTO_MIP_NODESEL_BANDIT="3")}
