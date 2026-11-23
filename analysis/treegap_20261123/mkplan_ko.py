# 部品を 1 つずつ外す計画 (打ち切り値 = 最適値 + 1e-6·max(1,|最適値|) を全部に与える)
import json
OPT = {"30n20b8": 302.0, "dano3_5": 576.9249159565586}
SCIP_KO = {
    "ko_pscost": ["branching/pscost/priority=1000000"],
    "ko_mostinf": ["branching/mostinf/priority=1000000"],
    "ko_nosym": ["misc/usesymmetry=0"],
    "ko_noconf": ["conflict/enable=FALSE"],
    "ko_notreesep": ["separating/maxrounds=0"],
    "ko_notreeprop": ["propagating/maxrounds=0"],
    "ko_nopseudoobj": ["propagating/pseudoobj/freq=-1"],
    "ko_noredcost": ["propagating/redcost/freq=-1"],
    "ko_novbounds": ["propagating/vbounds/freq=-1"],
    "ko_dfs": ["nodeselection/dfs/stdpriority=1000000"],
    "ko_bfs": ["nodeselection/bfs/stdpriority=1000000"],
    "ko_norestart": ["presolving/maxrestarts=0"],
    "ko_nocuts": ["separating/maxroundsroot=0", "separating/maxrounds=0"],
    "rootonly": ["limits/nodes=1"],
}
OURS_KO = {
    "ko_nosym": ["ENOMOTO_MIP_NO_SYMMETRY=1"],
    "ko_noconf": ["ENOMOTO_MIP_NO_CONFLICTS=1"],
    "ko_notreesep": ["ENOMOTO_MIP_NO_TREE_SEP=1"],
    "ko_pscost": ["ENOMOTO_T_MIP_MINREL=0"],
    "ko_dfsbb0": ["ENOMOTO_MIP_DFSBB=0"],
    "ko_norestart": ["ENOMOTO_MIP_NO_RESTART=1"],
    "rootsb_ilf": ["ENOMOTO_MIP_ROOTSB=ilf"],
}
if __name__ == "__main__":
    plan = []
    for p in ["30n20b8", "dano3_5"]:
        c = OPT[p] + 1e-6 * max(1, abs(OPT[p]))
        for t, a in SCIP_KO.items(): plan.append(["scip", p, 1, t, a + [f"OBJLIMIT={c}"]])
        for t, a in OURS_KO.items(): plan.append(["ours", p, 1, t, a + [f"ENOMOTO_MIP_CUTOFF={c}"]])
    json.dump(plan, open("plan_ko.json", "w"), indent=0)
    print(len(plan))
