"""HiGHS の双対単体法を、オプションを変えて解く (前処理なし、1 スレッド): python highs_opt.py LP名 キー=値 ...
摂動 (dual_simplex_cost_perturbation_multiplier)・価格付け (simplex_dual_edge_weight_strategy)・尺度 (simplex_scale_strategy) などの効き目を見る。"""
import json, sys, time
from pathlib import Path
import highspy
R = Path(__file__).resolve().parents[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.readModel(str(R / ".lpgap_cache" / f"{sys.argv[1]}.mps"))
opts = {"solver": "simplex", "simplex_strategy": 1, "threads": 1, "presolve": "off", "time_limit": 600.0}
for a in sys.argv[2:]:
    k, v = a.split("=")
    for f in (int, float, str):
        try:
            v = f(v); break
        except ValueError:
            pass
    opts[k] = v
for k, v in opts.items():
    h.setOptionValue(k, v)
t = time.perf_counter(); h.run(); t = time.perf_counter() - t
i = h.getInfo()
print(json.dumps({"lp": sys.argv[1], "opts": sys.argv[2:], "status": h.modelStatusToString(h.getModelStatus()), "obj": i.objective_function_value, "iters": i.simplex_iteration_count, "time": round(t, 2)}, ensure_ascii=False))
