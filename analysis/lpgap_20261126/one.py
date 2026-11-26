"""LP 1 本を当方か HiGHS で 1 スレッド・冷えた始動で解く: python one.py {ours|highs|highs_pre} LP名 [秒]
当方は lp_solver="simplex" (傾き・切片双対二段解法)。HiGHS は双対単体法 (simplex_strategy=1)、前処理は highs_pre だけ。
最後の行に JSON (時間・反復・目的値) を出す。当方の内訳は ENOMOTO_PROF_PHASES_EXT=1 などを環境変数で渡して stderr で見る。"""
import json, sys, time
from pathlib import Path
R = Path(__file__).resolve().parents[2]
who, name = sys.argv[1], sys.argv[2]; tl = float(sys.argv[3]) if len(sys.argv) > 3 else 600
f = R / ".lpgap_cache" / f"{name}.mps"
if who == "ours":
    import enomoto_solver as es
    M = es.Model(); M.readModel(str(f))
    t = time.perf_counter(); s = M.solve(lp_solver="simplex", threads=1, time_limit=tl, log=False); t = time.perf_counter() - t
    print(json.dumps({"who": who, "lp": name, "status": s.status, "obj": s.objective, "time": t}))
else:
    import highspy
    h = highspy.Highs()
    h.setOptionValue("output_flag", True); h.setOptionValue("log_to_console", True)
    h.readModel(str(f))
    for k, v in (("solver", "simplex"), ("simplex_strategy", 1), ("threads", 1), ("time_limit", tl),
                 ("presolve", "on" if who == "highs_pre" else "off"), ("highs_analysis_level", int(sys.argv[4]) if len(sys.argv) > 4 else 0)):
        h.setOptionValue(k, v)
    t = time.perf_counter(); h.run(); t = time.perf_counter() - t
    i = h.getInfo()
    print(json.dumps({"who": who, "lp": name, "status": h.modelStatusToString(h.getModelStatus()), "obj": i.objective_function_value,
                      "iters": i.simplex_iteration_count, "time": t}))
