# 追加 (親の依頼): mcsched の部品外し (30n20b8・dano3_5 と同じ組) と、pg での ENOMOTO_MIP_HEUR1T=fl
import json
from mkplan_ko import SCIP_KO, OURS_KO
opt = 211913.0; c = opt + 1e-6 * max(1, abs(opt))
plan = [["ours", "mcsched", 1, "cut", [f"ENOMOTO_MIP_CUTOFF={c}"]], ["scip", "mcsched", 1, "cut", [f"OBJLIMIT={c}"]]]
for t, a in SCIP_KO.items(): plan.append(["scip", "mcsched", 1, t, a + [f"OBJLIMIT={c}"]])
for t, a in OURS_KO.items(): plan.append(["ours", "mcsched", 1, t, a + [f"ENOMOTO_MIP_CUTOFF={c}"]])
for s in [1, 2, 3]:
    if s > 1: plan.append(["ours", "pg", s, "base", []])
    plan.append(["ours", "pg", s, "heur1t_fl", ["ENOMOTO_MIP_HEUR1T=fl"]])
json.dump(plan, open("plan_ko2.json", "w"), indent=0)
print(len(plan))
