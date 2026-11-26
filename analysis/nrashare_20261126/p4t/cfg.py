"""nrashare 8 節の P の比べ (2026-11-26、親の依頼): c2c と、c2c の ENUM_ADAPT を 1st にした形 (`t` を全部の前処理に広げたもの) を 4 スレッドで。
既定 (PAR4=fmr の後) は回さない (c2c と c2ct を同じビルドで比べる)。名前が _t1 で終われば 1 スレッド、そうでなければ 4 スレッド (run.py)。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("combo2_cfg", Path(__file__).resolve().parents[2] / "combo2_20261125" / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
C2C = dict(_m.C2C)
CFGS = {"c2c_t4": C2C, "c2ct_t4": dict(C2C, ENOMOTO_MIP_ENUM_ADAPT="1st")}
# summ.py の形: 試作は c2ct を 4 スレッドで、基準は c2c (BASE)
P1 = {1: ("c2ct", CFGS["c2ct_t4"], False, True, "nrashare_20261126")}
BASE = "c2c"
