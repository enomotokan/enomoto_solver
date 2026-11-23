"""HiGHS のヒューリスティクスを 1 つずつ切って 1 スレッドで解く。python highs_abl.py 問題 秒 [種,...] → logs/<問題>_highsabl.jsonl"""
import json, sys, time
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
prob, tl = sys.argv[1], float(sys.argv[2])
seeds = [int(s) for s in sys.argv[3].split(",")] if len(sys.argv) > 3 else [1]
CASES = {"def": {}, "no_rens": {"mip_heuristic_run_rens": False}, "no_rins": {"mip_heuristic_run_rins": False},
         "no_rrc": {"mip_heuristic_run_root_reduced_cost": False}, "no_fj": {"mip_heuristic_run_feasibility_jump": False},
         "no_zi": {"mip_heuristic_run_zi_round": False}, "no_shift": {"mip_heuristic_run_shifting": False},
         "no_heur": {"mip_heuristic_effort": 0.0, "mip_heuristic_run_feasibility_jump": False}}
for s in seeds:
    for name, opts in CASES.items():
        h = highspy.Highs(); h.setOptionValue("output_flag", False)
        h.setOptionValue("threads", 1); h.setOptionValue("time_limit", tl); h.setOptionValue("random_seed", s)
        for k, v in opts.items(): h.setOptionValue(k, v)
        h.readModel(str(WT / ".miplib_cache" / f"{prob}.mps"))
        t = time.time(); h.run(); el = time.time() - t
        info = h.getInfo()
        r = dict(problem=prob, seed=s, case=name, status=h.modelStatusToString(h.getModelStatus()), time=round(el, 2), obj=info.objective_function_value, nodes=info.mip_node_count)
        print(json.dumps(r), flush=True)
        with open(D / "logs" / f"{prob}_highsabl.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
