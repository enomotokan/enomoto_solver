"""PARALLEL_INT の確かめ (2026-11-13) の設定。基準は cand1114 (analysis/cand1114_20261113/cfg.py)、比べる側はそれに
ENOMOTO_PRESOLVE_PARALLEL_INT だけを足したもの。"""
import importlib.util
from pathlib import Path
_spec = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_spec); _spec.loader.exec_module(_m)
CAND1114 = _m.CAND1114
CFGS = {"cand1114": dict(CAND1114), "pint": {**CAND1114, "ENOMOTO_PRESOLVE_PARALLEL_INT": "1"}}
