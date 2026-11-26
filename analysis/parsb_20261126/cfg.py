"""parsb の試作の P の計測の設定。名前が _t4 なら 4 スレッド (run.py の TH)。
試作は `ENOMOTO_MIP_PARSB` (src/mip/solver.rs `parsb_flags`)。既定は PARIDLE=h が入った版。"""
P1 = {1: ("sb", {"ENOMOTO_MIP_PARSB": "1"}, False, True, "parsb_20261126")}
CFGS = {"def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
