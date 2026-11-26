"""nrashare の P の比べ (2026-11-26、親の依頼): c2c の nra を r0k23p にした形 (c2cp) と、それに ENOMOTO_MIP_NRA_SHARE=1 を足した形 (c2cps) を 4 スレッドで。
既定は回さない (c2cp と c2cps を同じビルドで比べる)。名前が _t1 で終われば 1 スレッド、そうでなければ 4 スレッド (run.py)。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("combo2_cfg", Path(__file__).resolve().parents[2] / "combo2_20261125" / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
C2CP = dict(_m.C2C, ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p")
CFGS = {"c2cp_t4": C2CP, "c2cps_t4": dict(C2CP, ENOMOTO_MIP_NRA_SHARE="1")}
# summ.py の形: 試作は c2cps を 4 スレッドで、基準は c2cp (BASE)
P1 = {1: ("c2cps", CFGS["c2cps_t4"], False, True, "nrashare_20261126")}
BASE = "c2cp"
