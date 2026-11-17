"""SUBMIP_HALT の確かめ (analysis/timedep_20261114.md の 9 節、4 スレッド・60 秒、58 問 × 種 1〜4)。束は analysis/cand1115_20261115/cfg.py。
base = CAND1115C (TIMEDEP なし、今の束)、cH = base + SUBMIP_HALT、tdH = CAND1115 (TIMEDEP Et2 入り) + SUBMIP_HALT。
tbfp-network (600 秒) の損は、早めの再スタートで止めの合図を送ったサブ MIP が長い LP の途中で合図を見なかったことだった (SUBMIP_HALT で直る)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1115", Path(__file__).resolve().parents[2] / "cand1115_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
H = {"ENOMOTO_MIP_SUBMIP_HALT": "1"}
CFGS = {"base": dict(_m.CAND1115C), "cH": {**_m.CAND1115C, **H}, "tdH": {**_m.CAND1115, **H}}
CFGS_ALL = CFGS
