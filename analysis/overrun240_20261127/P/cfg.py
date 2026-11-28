"""裏の研磨 `ENOMOTO_MIP_POLISH_BG` (src/mip/solver.rs `polish_bg`) の P の計測の設定。名前が _t2 なら 2 スレッド、_t4 なら 4 スレッド (run.py の TH)。
P1 の値は (名前, 環境変数, 2 スレッドで回すか, 4 スレッドで回すか, 記録の場所)。"""
P1 = {1: ("i", {"ENOMOTO_MIP_POLISH_BG": "i"}, True, True, "overrun240_20261127")}
CFGS = {"def_t2": {}, "def_t4": {}}
for k, (name, env, t2, t4, _) in P1.items():
    if t2: CFGS[f"{name}_t2"] = env
    if t4: CFGS[f"{name}_t4"] = env
