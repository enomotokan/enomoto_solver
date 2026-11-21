"""SCIP・HiGHS を 1 問だけ回す子プロセス (run.py から呼ぶ)。
使い方: python one_other.py scip|highs MPS 時間の上限 ログ 統計 [nodes]
- scip: 1 スレッド (逐次)、種 1、ログを setLogfile、終わったら writeStatistics。nodes を渡すと limits/nodes (1 なら根で止める)。
- highs: 既定のスレッド、log_file にログ。nodes を渡すと mip_max_nodes。
最後に 1 行の JSON (状態・時間・下界・暫定解・ノード) を stdout に出す。"""
import json, sys, time
solver, mps, tl, log, stat = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4], sys.argv[5]
nodes = int(sys.argv[6]) if len(sys.argv) > 6 else None
t0 = time.time()
if solver == "scip":
    import pyscipopt
    m = pyscipopt.Model()
    m.setLogfile(log)
    m.readProblem(mps)
    m.setParam("limits/time", tl)
    m.setParam("randomization/randomseedshift", 1)
    m.setParam("display/freq", 1)
    if nodes: m.setParam("limits/nodes", nodes)
    m.optimize()
    m.writeStatistics(stat)
    rec = {"status": m.getStatus(), "time": m.getSolvingTime(), "bound": m.getDualbound(), "objective": m.getPrimalbound() if m.getNSols() else None,
           "nodes": m.getNNodes(), "root_lp_iters": None, "lp_iters": m.getNLPIterations()}
else:
    import highspy
    h = highspy.Highs()
    h.setOptionValue("log_file", log)
    h.setOptionValue("log_to_console", False)
    h.setOptionValue("time_limit", tl)
    h.setOptionValue("random_seed", 1)
    if nodes: h.setOptionValue("mip_max_nodes", nodes)
    h.readModel(mps)
    h.run()
    info = h.getInfo()
    rec = {"status": h.modelStatusToString(h.getModelStatus()), "time": h.getRunTime(), "bound": info.mip_dual_bound,
           "objective": info.objective_function_value if info.primal_solution_status else None, "nodes": info.mip_node_count,
           "lp_iters": info.simplex_iteration_count}
rec["wall"] = time.time() - t0
print(json.dumps(rec), flush=True)
