"""cand1113 の leave-one-out その 2 (2026-11-13)。Mittelmann 600 秒で 2 種とも遅い enlight_hard・neos-1122047・cbs-cta 用。
形は ../loo/cfg.py と同じ (def・cand1113・まとまり X を外した "-X" の 20 形)。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg_loo", Path(__file__).resolve().parents[1] / "loo" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CFGS = {c: v for c, v in _m.CFGS.items() if not c.startswith("-P:")}
