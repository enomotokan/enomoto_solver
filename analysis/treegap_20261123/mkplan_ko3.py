# 30n20b8 で強分枝まわりを当方で試す (打ち切り値つき)。LOG つきは SCIP の統計と比べるための記録
import json
c = 302.0 + 1e-6 * 302.0
C = [f"ENOMOTO_MIP_CUTOFF={c}"]
plan = [["ours", "30n20b8", 1, "cut_log", C + ["ENOMOTO_MIP_LOG=1"]],
        ["ours", "30n20b8", 1, "sb_fixres", C + ["ENOMOTO_MIP_SB_FIX_RESOLVE=1", "ENOMOTO_MIP_LOG=1"]],
        ["ours", "30n20b8", 1, "sb_tighten", C + ["ENOMOTO_MIP_SB_TIGHTEN=1"]],
        ["ours", "30n20b8", 1, "minrel16", C + ["ENOMOTO_T_MIP_MINREL=16"]],
        ["ours", "30n20b8", 1, "minrel32", C + ["ENOMOTO_T_MIP_MINREL=32"]]]
json.dump(plan, open("plan_ko3.json", "w"), indent=0)
