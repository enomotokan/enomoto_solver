"""b58f の 5 つの設定 (analysis/treeclose_20261114.md 9.8 節、利用者の案)。すべて束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116、TC=bs + SUBMIP_HALT) の上。
base: cand1116 のまま。
d1: 今の深さ優先寄り (PLUNGE_QUOT=1、SIBLING_QUOT=1、PLUNGE_NODES_MAX=1e9、PLUNGE_NODES_DIV=1) = cand1116f。
d2: d1 で SIBLING_QUOT を既定 (0.25) に戻す (子へは潜り続け、行き止まりで兄弟の下界が悪ければ全体の最良のノードへ戻る)。
d3: d1 で PLUNGE_NODES_MAX・PLUNGE_NODES_DIV を既定 (1000・10) に戻す。
d4: d1 で SIBLING_QUOT・PLUNGE_NODES_MAX・PLUNGE_NODES_DIV を既定に戻す (= PLUNGE_QUOT=1 だけ)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
PQ = {"ENOMOTO_T_MIP_PLUNGE_QUOT": "1"}
SQ = {"ENOMOTO_T_MIP_SIBLING_QUOT": "1"}
NM = {"ENOMOTO_T_MIP_PLUNGE_NODES_MAX": "1000000000", "ENOMOTO_T_MIP_PLUNGE_NODES_DIV": "1"}
CFGS = {
    "base": BASE,
    "d1": dict(BASE, **PQ, **SQ, **NM),
    "d2": dict(BASE, **PQ, **NM),
    "d3": dict(BASE, **PQ, **SQ),
    "d4": dict(BASE, **PQ),
}
