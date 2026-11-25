"""目的の伝播の抜けの直し (analysis/objprop_20261124) の P の設定。名前が _t1 で終われば 1 スレッド、そうでなければ 4 スレッド (run.py の TH)。
P1 の 6 番目は比べる相手 (既定 def のほかに、その設定とも比べる。summ.py)。"""
OG = {"ENOMOTO_MIP_ROOT_OBJPROP_GLOBAL": "1"}
GF = {"ENOMOTO_MIP_GLOBALFIX": "abc"}
P1 = {  # 番号: (名前, 環境変数, 1 スレッド, 4 スレッド, 記録, もう 1 つの比べる相手)
    1: ("og", OG, True, True, "objprobehull_20261124 7 節", None),
    2: ("ogop", {**OG, "ENOMOTO_MIP_OBJ_PROP": "1"}, True, True, "O1 (ノードの目的の伝播)", "og"),
    3: ("redo", {**OG, "ENOMOTO_MIP_OBJPROP_REDO": "gp"}, True, False, "O6", "og"),
    4: ("sbpb", {**OG, "ENOMOTO_MIP_SB_PROBEBD": "1"}, True, False, "O3 の土台 (強分枝の子の伝播)", "og"),
    5: ("sbpbop", {**OG, "ENOMOTO_MIP_SB_PROBEBD": "1", "ENOMOTO_MIP_SB_OBJPROP": "1"}, True, False, "O3", "sbpb"),
    # 第 2 段 (shards2.json、plan2.py): 連続列を締めない目的の伝播 (OBJ_PROP_INT) と、監査の GLOBALFIX=abc との組み合わせ
    6: ("ogopi", {**OG, "ENOMOTO_MIP_OBJ_PROP": "1", "ENOMOTO_MIP_OBJ_PROP_INT": "1"}, True, True, "O1 (連続列を締めない)", "ogop"),
    7: ("sbpbopi", {**OG, "ENOMOTO_MIP_SB_PROBEBD": "1", "ENOMOTO_MIP_SB_OBJPROP": "1", "ENOMOTO_MIP_OBJ_PROP_INT": "1"}, True, False, "O3 (連続列を締めない)", "sbpb"),
    8: ("gf", GF, True, True, "監査の A1・A2・C1 (globalfix_20261124)", None),
    9: ("gfog", {**GF, **OG}, True, True, "GLOBALFIX + O5", "gf"),
    10: ("gfogop", {**GF, **OG, "ENOMOTO_MIP_OBJ_PROP": "1", "ENOMOTO_MIP_OBJ_PROP_INT": "1"}, True, True, "GLOBALFIX + O5 + O1 (連続列を締めない)", "gfog"),
}
CFGS = {"def_t1": {}, "def_t4": {}}
for k, (name, env, t1, t4, _, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
