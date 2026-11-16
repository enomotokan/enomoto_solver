"""b58d の 2 つの設定 (analysis/treeclose_20261114.md 9 章)。base = 束 cand1115 (analysis/cand1115_20261115/cfg.py、TC=b)。
dfs = base + 深さ優先寄り (潜りの打ち切りを外す) だけ。mcsched (1 スレッド・打ち切り値つき) で深さ優先寄りだけでも 902 秒で閉じ、
TC=b (束と同じ) と組んで 761 秒、TC=as 894 秒、TC=as + 基底の退化した列の固定 1042 秒だったので、TC は束のまま (b)、退化した列の固定は入れない。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1115", Path(__file__).resolve().parents[2] / "cand1115_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1115)
DF = {"ENOMOTO_T_MIP_PLUNGE_QUOT": "1", "ENOMOTO_T_MIP_SIBLING_QUOT": "1", "ENOMOTO_T_MIP_PLUNGE_NODES_MAX": "1000000000", "ENOMOTO_T_MIP_PLUNGE_NODES_DIV": "1"}
DD = {"ENOMOTO_MIP_NODECNT": "hrd", "ENOMOTO_T_MIP_NODECNT_MAXC": "5000"}
CFGS = {"base": BASE, "dfs": dict(BASE, **DF)}
