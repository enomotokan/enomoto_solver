"""SCIP のヒューリスティクスを入り切りして、時間・ノード・最初の解の出どころを並べる (切り分け)。
python scip_heur.py <問題> <種,...> <秒> [ケース,...] → logs/<問題>_sciph.jsonl に 1 行ずつ足す"""
import sys, json
from pathlib import Path
from pyscipopt import Model
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, seeds, tl = sys.argv[1], [int(s) for s in sys.argv[2].split(",")], float(sys.argv[3])
CASES = {
    "default": {},
    "no_lpface": {"heuristics/lpface/freq": -1},
    "no_rens": {"heuristics/rens/freq": -1},
    "no_lns": {"heuristics/lpface/freq": -1, "heuristics/rens/freq": -1, "heuristics/rins/freq": -1, "heuristics/alns/freq": -1,
               "heuristics/crossover/freq": -1, "heuristics/dins/freq": -1, "heuristics/gins/freq": -1, "heuristics/localbranching/freq": -1,
               "heuristics/mutation/freq": -1, "heuristics/trustregion/freq": -1},
    "lpface_root": {"heuristics/lpface/minpathlen": 0},
    "lpf_root_nolprows": {"heuristics/lpface/minpathlen": 0, "heuristics/lpface/uselprows": False, "heuristics/lpface/copycuts": False},
    "lpf_root_obj": {"heuristics/lpface/minpathlen": 0, "heuristics/lpface/subscipobjective": "o"},
    "lpf_root_nocut": {"heuristics/lpface/minpathlen": 0, "separating/maxroundsroot": 0},
    "lpf_root_dbe": {"heuristics/lpface/minpathlen": 0, "heuristics/lpface/dualbasisequations": True},
    "rens_nocut": {"separating/maxroundsroot": 0},
    "rens_lprows": {"heuristics/rens/uselprows": True},
    "rens_minfix": {"heuristics/rens/minfixingrate": 0.3},
    "no_heur": {"heuristics/emphasis": None},
}
sel = sys.argv[4].split(",") if len(sys.argv) > 4 else list(CASES)
for seed in seeds:
    for name in sel:
        m = Model(); m.hideOutput(); m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps"))
        m.setParam("limits/time", tl); m.setParam("randomization/randomseedshift", seed)
        for k, v in CASES[name].items():
            if v is None:
                from pyscipopt import SCIP_PARAMSETTING
                m.setHeuristics(SCIP_PARAMSETTING.OFF)
            else:
                m.setParam(k, v)
        m.optimize()
        first = None
        sols = m.getSols()
        heurs = []
        for s in sols:
            h = s.getHeur() if hasattr(s, "getHeur") else None
            try:
                heurs.append(h.getName() if h is not None else "relax/lp")
            except Exception:
                heurs.append(str(h))
        row = {"case": name, "seed": seed, "status": m.getStatus(), "time": round(m.getSolvingTime(), 2), "nodes": m.getNNodes(),
               "obj": m.getObjVal() if m.getNSols() else None, "dual": m.getDualbound(),
               "sols_by": heurs[:5]}
        with open(D / "logs" / f"{p}_sciph.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(json.dumps(row), flush=True)
