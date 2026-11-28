"""heurfp の第 2 段 (MIPLIB 240 問、analysis/heurfp_20261128/s2) の設定: 既定と heurfp (HEUR_ADAPT=1mkr + FP_STAGE3_LS=a、../cfg.py の HEURFP)。
名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド。"""
HEURFP = {"ENOMOTO_MIP_HEUR_ADAPT": "1mkr", "ENOMOTO_MIP_FP_STAGE3_LS": "a"}
CFGS = {"def_t1": {}, "def_t4": {}, "heurfp_t1": dict(HEURFP), "heurfp_t4": dict(HEURFP)}
