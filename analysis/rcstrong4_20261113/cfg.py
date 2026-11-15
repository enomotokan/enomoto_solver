"""RCSTRONG4 (2026-11-13): 試作 `ENOMOTO_MIP_RCSTRONG=gk` (i の根の切り捨てを保留し、根の再スタートを決めたときだけ当てる、
src/mip/rcstrong.rs) を、束 cand1114 (RCSTRONG=k) の上で k・ik と比べる。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
CFGS = {"def": {}, "k": dict(CAND1114), "ik": dict(CAND1114, ENOMOTO_MIP_RCSTRONG="ik"), "gk": dict(CAND1114, ENOMOTO_MIP_RCSTRONG="gk")}
