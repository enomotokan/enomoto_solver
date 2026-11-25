"""組み直した組み合わせの第 2 段 (MIPLIB 240 問、analysis/combo2_20261125/s2) の設定: 既定と c2c (第 1 段の第一候補、親の判断)。
c2c の中身は第 1 段の cfg.py (analysis/combo2_20261125/cfg.py) の C2C をそのまま使う。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("combo2_cfg", Path(__file__).resolve().parents[1] / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
C2C = _m.C2C
CFGS = {"def_t1": {}, "def_t4": {}, "c2c_t1": dict(C2C), "c2c_t4": dict(C2C)}
