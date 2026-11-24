"""試作の第 1 段のふるい分け (P、analysis/protoscreen_20261124) の設定。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
既定 (def_t1・def_t4) は 1 回だけ回し、全試作と共有する。環境変数の名前と値は各記録とコードで確かめた (README 1 節)。"""
P1 = {  # 試作の番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("h1t", {"ENOMOTO_MIP_HEUR1T": "fl"}, True, False, "heur1t_20261123"),
    2: ("fps", {"ENOMOTO_MIP_FP_STAGE3": "1", "ENOMOTO_MIP_POOR_NOINC": "1", "ENOMOTO_MIP_P2RIT": "a", "ENOMOTO_MIP_FJSYNC_LABEL": "1"}, True, False, "presym_20261123"),
    3: ("enum", {"ENOMOTO_MIP_ENUM_PRESOLVE": "all", "ENOMOTO_MIP_ENUM_WHERE": "loop", "ENOMOTO_MIP_PRR_TIGHT": "1"}, True, True, "cutgap_20261123"),
    4: ("ncl", {"ENOMOTO_MIP_NODE_CLEANUP": "qa"}, True, True, "nodespeed_20261123"),
    5: ("pool1", {"ENOMOTO_MIP_POOL_SEL": "1"}, True, True, "nodespeed_20261123"),
    6: ("poole", {"ENOMOTO_MIP_POOL_SEL": "e"}, True, True, "nodespeed_20261123"),
    7: ("hso", {"ENOMOTO_MIP_HO4_ACFIX": "d", "ENOMOTO_MIP_SB_NO_DEGEN_SKIP": "1", "ENOMOTO_MIP_SB_BUDGET_HIGHS": "1"}, True, True, "highsonly_20261123"),
    8: ("lux", {"ENOMOTO_MIP_LU_EXTEND": "1"}, True, True, "lpiter_20261123"),
    9: ("bfrt", {"ENOMOTO_MIP_NODELP_BFRT_GROUP": "1"}, True, True, "lpiter_20261123"),
    10: ("ppi", {"ENOMOTO_MIP_PROOFPROP": "i"}, False, True, "proofprop_20261123"),
    11: ("sbpb", {"ENOMOTO_MIP_SB_PROBEBD": "1"}, True, True, "treegap_20261123"),
}
CFGS = {"def_t1": {}, "def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
