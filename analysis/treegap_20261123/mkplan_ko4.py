# SCIP の relpscost の点数付け (退化したノードで擬似費用の点数を退化の度合いで割る・衝突の重み 0.01) を既存の切り替えで試す
import json
OPT = {"30n20b8": 302.0, "dano3_5": 576.9249159565586, "mcsched": 211913.0, "swath1": 379.07129575}
CF = {"degscore": ["ENOMOTO_MIP_S4_DEGSCORE=1"],
      "confw": ["ENOMOTO_T_MIP_CONFLICT_WEIGHT=0.01"],
      "degscore_confw": ["ENOMOTO_MIP_S4_DEGSCORE=1", "ENOMOTO_T_MIP_CONFLICT_WEIGHT=0.01"],
      "nodegskip": ["ENOMOTO_MIP_SB_NO_DEGEN_SKIP=1"]}
plan = [["ours", "swath1", 1, "cut", [f"ENOMOTO_MIP_CUTOFF={OPT["swath1"] + 1e-6 * OPT["swath1"]}"]]]
for p, o in OPT.items():
    c = o + 1e-6 * max(1, abs(o))
    for t, a in CF.items(): plan.append(["ours", p, 1, t, a + [f"ENOMOTO_MIP_CUTOFF={c}"]])
json.dump(plan, open("plan_ko4.json", "w"), indent=0); print(len(plan))
