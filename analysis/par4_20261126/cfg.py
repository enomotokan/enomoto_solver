"""par4 の試作の P の計測 (README 5 節) の設定。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
どれも既定 (版 27a8f25eb 以降) に足す。試作は `ENOMOTO_MIP_PAR4` の文字 (src/mip/solver.rs `par4`)。"""
P1 = {1: ("mr", {"ENOMOTO_MIP_PAR4": "mr"}, False, True, "par4_20261126")}
CFGS = {"def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
