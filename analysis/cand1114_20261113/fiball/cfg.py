"""fiball などの切り分け (2026-11-13): cand1113 で効いて cand1114 で消えた/解けなくなった問題で、cand1113 → cand1114 の差を 1 つずつ戻す。
"+X" は CAND1114 に cand1113 の項目 X を足した (戻した) 形。"""
import importlib.util
from pathlib import Path
_d = Path(__file__).resolve().parents[1]
def _load(name, path):
    sp = importlib.util.spec_from_file_location(name, path); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m); return m
CAND1114 = _load("cfg1114", _d / "cfg.py").CAND1114
CAND1113 = _load("cfg1113", _d.parent / "cand1113_20261112" / "cfg.py").CAND1113
BACK = {
    "+RCSTRONG_irk": {"ENOMOTO_MIP_RCSTRONG": "irk"},
    "+HNFPRE0": {"ENOMOTO_MIP_HNFPRE": "0"},
    "+SUBFIX0": {"ENOMOTO_MIP_SUBFIX": "0"},
    "+utw": {"ENOMOTO_MIP_IPMRC": "utw"},
    "+DUALEQ": {"ENOMOTO_MIP_DUALEQ": "1"},
    "+CLQPRB": {"ENOMOTO_MIP_CLQPRB": "1d"},
}
CFGS = {"def": {}, "cand1113": CAND1113, "cand1114": CAND1114}
for _k, _v in BACK.items():
    CFGS[_k] = dict(CAND1114, **_v)
# 6 つを全部戻すと cand1113 と同じになる
assert dict(CAND1114, **{k: v for e in BACK.values() for k, v in e.items()}) == CAND1113
