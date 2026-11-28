"""heurfp (HEUR_ADAPT=1mkr + FP_STAGE3_LS=a) を 600 秒で確かめる (analysis/heurfp_20261128、2026-11-28、親の依頼・利用者の承認)。
既定は enum・nra の既定化 (f281fa7b8) の後の形。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。"""
HEURFP = {"ENOMOTO_MIP_HEUR_ADAPT": "1mkr", "ENOMOTO_MIP_FP_STAGE3_LS": "a"}
P1 = {1: ("heurfp", HEURFP, True, True, "heurfp_20261128")}
CFGS = {"def_t1": {}, "def_t4": {}, "heurfp_t1": dict(HEURFP), "heurfp_t4": dict(HEURFP)}
