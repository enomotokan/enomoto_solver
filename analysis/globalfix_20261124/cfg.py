"""globalfix の計測の設定 (analysis/protoscreen_20261124/cfg.py の形)。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("abc", {"ENOMOTO_MIP_GLOBALFIX": "abc"}, True, True, "globalfix_20261124"),
    2: ("a", {"ENOMOTO_MIP_GLOBALFIX": "a"}, True, True, "globalfix_20261124"),
    3: ("b", {"ENOMOTO_MIP_GLOBALFIX": "b"}, True, True, "globalfix_20261124"),
    4: ("c", {"ENOMOTO_MIP_GLOBALFIX": "c"}, True, True, "globalfix_20261124"),
}
CFGS = {"def_t1": {}, "def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
