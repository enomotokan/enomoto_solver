"""TREEGAP (2026-11-23) の 1 回を回して res_tg.jsonl に足す (analysis/treegap_20261121/tg.py の写し + HiGHS、README 参照)。ソルバーは同時に 1 つ、1 スレッド、上限 600 秒。
  python tg.py ours PROB SEED TAG [ENOMOTO_X=V ...]   当方 (環境変数で切り替え)
  python tg.py scip PROB SEED TAG [param=value ...]    SCIP (pyscipopt、パラメータで切り替え。値は int/float/bool/str を推す。
                                                       OBJLIMIT=v は setObjlimit。統計は logs/PROB_sSEED_TAG.stats)
  python tg.py highs PROB SEED TAG [option=value ...]  HiGHS (highspy、オプションで切り替え。OBJLIMIT=v は objective_bound)
  python tg.py run PLAN.json                            計画 ([[solver, prob, seed, tag, [args...]], ...]) を順に回す。記録済み (solver, prob, seed, tag) は飛ばす
TG_TL で上限秒を変えられる (既定 600)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
OUT = D / "res_tg.jsonl"
TL = float(os.environ.get("TG_TL", "600"))


def solu():
    ref = {}
    for l in open(WT / "scripts/miplib2017-v31.solu"):
        t = l.split()
        if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
    return ref


def conv(v):
    if v in ("TRUE", "true", "True"): return True
    if v in ("FALSE", "false", "False"): return False
    for f in (int, float):
        try: return f(v)
        except ValueError: pass
    return v


def worker_scip(mps, seed, params):
    import pyscipopt
    m = pyscipopt.Model(); m.hideOutput()
    m.setParam("limits/time", TL); m.setParam("limits/gap", 0.0)
    if seed: m.setParam("randomization/randomseedshift", seed)
    m.readProblem(mps)
    for k, v in params.items():
        if k == "OBJLIMIT": m.setObjlimit(float(v))
        else: m.setParam(k, conv(v))
    t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
    if os.environ.get("TG_STATS"): m.writeStatistics(os.environ["TG_STATS"])
    has = m.getNSols() > 0
    print(json.dumps({"status": m.getStatus(), "objective": m.getObjVal() if has else None, "bound": m.getDualbound(),
                      "root_bound": m.getDualboundRoot(), "nodes": m.getNTotalNodes(), "time": t,
                      "lp_iterations": m.getNLPIterations(), "max_depth": m.getMaxDepth()}))


def worker_highs(mps, seed, params):
    import highspy
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", TL); h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("threads", 1)
    h.setOptionValue("random_seed", seed)
    for k, v in params.items():
        if k == "OBJLIMIT": h.setOptionValue("objective_bound", float(v))
        else: h.setOptionValue(k, conv(v))
    h.readModel(mps)
    t = time.perf_counter(); h.run(); t = time.perf_counter() - t
    info = h.getInfo(); st = h.modelStatusToString(h.getModelStatus())
    has = info.primal_solution_status == 2
    print(json.dumps({"status": st, "objective": info.objective_function_value if has else None, "bound": info.mip_dual_bound,
                      "root_bound": None, "nodes": info.mip_node_count, "time": t, "lp_iterations": None}))


def worker_ours(mps):
    import run_miplib_benchmark as rb
    from enomoto_solver import benchmark_highs as bh
    m, _, _, _ = rb._read_our_model_compat(bh, Path(mps))
    t = time.perf_counter(); r = m.solve(None, False, TL, None, None, 1); t = time.perf_counter() - t
    print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"], "root_bound": r.get("root_bound"),
                      "nodes": r["nodes"], "time": t, "lp_iterations": r.get("lp_iterations"),
                      "first_solution_time": r.get("first_solution_time"), "first_solution_objective": r.get("first_solution_objective")}))


def one(solver, prob, seed, tag, args):
    import run_miplib_benchmark as rb
    mps = rb.fetch(prob, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    params = {}
    for a in args:
        k, v = a.split("=", 1)
        if solver == "ours": env[k] = v
        else: params[k] = v
    env["ENOMOTO_MIP_THREADS"] = "1"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    if solver == "scip":  # SCIP の統計 (分離・伝播・衝突の表) を logs/ に残す
        (D / "logs").mkdir(exist_ok=True); env["TG_STATS"] = str(D / "logs" / f"{prob}_s{seed}_{tag}.stats")
    t0 = time.time()
    pr = subprocess.run([sys.executable, __file__, "_w" + solver, str(mps), str(seed), json.dumps(params)], cwd=WT, env=env,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=TL + 300)
    if solver == "ours" and "ENOMOTO_MIP_LOG" in env:  # 当方の記録 (標準エラー) を logs/ に残す
        (D / "logs").mkdir(exist_ok=True); (D / "logs" / f"{prob}_s{seed}_{tag}.ourslog").write_text(pr.stderr)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode, "err": pr.stderr[-500:]}  # noqa: BLE001
    row = {"solver": solver, "problem": prob, "seed": seed, "tag": tag, "args": args, "tl": TL, "result": res,
           "wall": round(time.time() - t0, 1), "opt": solu().get(prob), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(solver, prob, seed, tag, res.get("status"), res.get("nodes"), round(res.get("time") or 0, 1), res.get("root_bound"), flush=True)


if __name__ == "__main__":
    a = sys.argv[1:]
    if a[0] == "_wscip": worker_scip(a[1], int(a[2]), json.loads(a[3]))
    elif a[0] == "_wours": worker_ours(a[1])
    elif a[0] == "_whighs": worker_highs(a[1], int(a[2]), json.loads(a[3]))
    elif a[0] == "run":
        done = set()
        if OUT.exists():
            for l in OUT.read_text().splitlines():
                if l.strip(): r = json.loads(l); done.add((r["solver"], r["problem"], r["seed"], r["tag"]))
        for s, p, sd, tag, args in json.loads(Path(a[1]).read_text()):
            if (s, p, sd, tag) not in done: one(s, p, sd, tag, args)
    else: one(a[0], a[1], int(a[2]), a[3], a[4:])
