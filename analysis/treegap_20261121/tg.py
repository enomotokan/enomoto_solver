"""TREEGAP の 1 回を回して res_tg.jsonl に足す (README 参照)。ソルバーは同時に 1 つ、1 スレッド、上限 600 秒。
  python tg.py ours PROB SEED TAG [ENOMOTO_X=V ...]   当方 (環境変数で切り替え)
  python tg.py scip PROB SEED TAG [param=value ...]    SCIP (pyscipopt、パラメータで切り替え。値は int/float/bool/str を推す)
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
        if k == "SET_EMPHASIS": continue
        m.setParam(k, conv(v))
    t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
    has = m.getNSols() > 0
    print(json.dumps({"status": m.getStatus(), "objective": m.getObjVal() if has else None, "bound": m.getDualbound(),
                      "root_bound": m.getDualboundRoot(), "nodes": m.getNTotalNodes(), "time": t,
                      "lp_iterations": m.getNLPIterations(), "max_depth": m.getMaxDepth()}))


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
    t0 = time.time()
    pr = subprocess.run([sys.executable, __file__, "_w" + solver, str(mps), str(seed), json.dumps(params)], cwd=WT, env=env,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=TL + 300)
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
    elif a[0] == "run":
        done = set()
        if OUT.exists():
            for l in OUT.read_text().splitlines():
                if l.strip(): r = json.loads(l); done.add((r["solver"], r["problem"], r["seed"], r["tag"]))
        for s, p, sd, tag, args in json.loads(Path(a[1]).read_text()):
            if (s, p, sd, tag) not in done: one(s, p, sd, tag, args)
    else: one(a[0], a[1], int(a[2]), a[3], a[4:])
