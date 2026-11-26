"""c2c v3 の第 2 段 (MIPLIB 240 問、analysis/combo3_20261126/s2) の設定: 既定と c3。c3 の中身は第 1 段の cfg.py (../cfg.py) の C3 をそのまま使う
(部品を外すときは ../cfg.py の USE を直す)。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("combo3_cfg", Path(__file__).resolve().parents[1] / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
C3 = _m.C3
CFGS = {"def_t1": {}, "def_t4": {}, "c3_t1": dict(C3), "c3_t4": dict(C3)}
