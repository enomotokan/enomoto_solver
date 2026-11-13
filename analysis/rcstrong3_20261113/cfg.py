"""RCSTRONG3 (2026-11-13): 束 cand1114 の RCSTRONG を決め直す。土台は CAND1114 (analysis/cand1114_20261113/cfg.py、RCSTRONG=k)。
形: k (今)・ik (ノードでも切り捨てる)・irk (根だけで切り捨てる、cand1113 の値)。def は環境変数なし。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
assert CAND1114["ENOMOTO_MIP_RCSTRONG"] == "k"
CFGS = {"def": {}, "k": dict(CAND1114), "ik": dict(CAND1114, ENOMOTO_MIP_RCSTRONG="ik"), "irk": dict(CAND1114, ENOMOTO_MIP_RCSTRONG="irk")}
