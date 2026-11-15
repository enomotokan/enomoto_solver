"""rootheur の設定。def・cand1114 は analysis/cand1114_20261113/cfg.py と同じ。試作の組は cand1114 + 試作の環境変数。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
CAND = _m.CFGS["cand1114"]
CFGS = {"def": {}, "cand1114": CAND}
for _k, _v in {"re": "e", "rb": "b", "rs": "s", "res": "es"}.items():
    CFGS[_k] = dict(CAND, ENOMOTO_MIP_RHEUR=_v)
