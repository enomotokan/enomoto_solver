"""b58e の 2 つの設定 (analysis/treeclose_20261114.md 9.3 節)。base = 束 cand1116f (analysis/cand1116_20261115/cfg.py の CAND1116F、深さ優先寄り入り)。
dmx = base + 潜りの深さの上限を外す (`ENOMOTO_T_MIP_PLUNGE_DEPTH_MAX` = 1e9、既定 200)。上限 200 は待ち行列から取り出すまでの潜りの通算で数えるので、
深い木 (sct2・uccase12) では深さ優先寄りでも途中で潜らなくなり、開いたノードが溜まっていた。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116F)
CFGS = {"base": BASE, "dmx": dict(BASE, ENOMOTO_T_MIP_PLUNGE_DEPTH_MAX="1000000000")}
