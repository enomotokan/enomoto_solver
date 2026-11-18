"""SCIP を 1 つずつ設定を変えて解き、時間・ノード・最初の根の LP 値・前処理後の列数を並べる (切り分け)。
python scip_abl.py <問題> <種> <秒> → logs/<問題>_scipabl.jsonl に 1 行ずつ足す"""
import sys, json
from pathlib import Path
from pyscipopt import Model
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, seed, tl = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
CASES = {
    "default": {},
    "no_vbounds_pre": {"propagating/vbounds/maxprerounds": 0},
    "no_orbpp_prop": {"constraints/orbitope_pp/propfreq": -1},
    "no_orbpp_pre": {"constraints/orbitope_pp/maxprerounds": 0},
    "no_logicor_dual": {"constraints/logicor/dualpresolving": False},
    "no_setppc_dual": {"constraints/setppc/dualpresolving": False},
    "no_dualreds": {"misc/allowstrongdualreds": False, "misc/allowweakdualreds": False},
    "no_probing": {"propagating/probing/maxprerounds": 0},
    "no_cuts": {"separating/maxrounds": 0, "separating/maxroundsroot": 0},
    "no_sym": {"misc/usesymmetry": 0},
    "no_rapidlearning": {"separating/rapidlearning/freq": -1},
    "no_conflict": {"conflict/enable": False},
}
sel = sys.argv[4].split(",") if len(sys.argv) > 4 else list(CASES)
for name in sel:
    m = Model(); m.hideOutput(); m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps"))
    m.setParam("limits/time", tl); m.setParam("randomization/randomseedshift", seed)
    for k, v in CASES[name].items():
        m.setParam(k, v)
    m.optimize()
    row = {"case": name, "status": m.getStatus(), "time": round(m.getSolvingTime(), 2), "nodes": m.getNNodes(),
           "obj": m.getObjVal() if m.getNSols() else None, "dual": m.getDualbound(), "firstlp": m.getFirstLpTime() if hasattr(m, "getFirstLpTime") else None,
           "presolved_vars": m.getNVars(True)}
    with open(D / "logs" / f"{p}_scipabl.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
