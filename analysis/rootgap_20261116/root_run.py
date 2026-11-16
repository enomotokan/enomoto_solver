"""根の比べのログを取る (analysis/rootgap_20261116/README.md): python root_run.py 問題,... [highs|ours|both] [K=V ...]
- highs: highspy (HiGHS 1.15.1)、4 スレッド・600 秒・mip_max_nodes=1 (根まで)、ログを logs/highs_<問題>.log に。
- ours: CAND1118 (analysis/cand1118_20261116/cfg.py) + K=V、ENOMOTO_MIP_LOG=1、4 スレッド・600 秒・node_limit=1、ログを logs/ours<札>_<問題>.log に
  (札は環境変数 TAG、既定は空)。1 問ずつ順に解く (同時に 1 つ)。結果の 1 行を res.jsonl に足す。"""
import json, os, subprocess, sys, time, importlib.util
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sp = importlib.util.spec_from_file_location("cfg1118", WT / "analysis/cand1118_20261116/cfg.py"); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m)
probs = sys.argv[1].split(","); which = sys.argv[2] if len(sys.argv) > 2 else "both"; kv = [a for a in sys.argv[3:] if "=" in a]
TAG = os.environ.get("TAG", "")
HCODE = """
import json, sys, time, highspy
h = highspy.Highs()
h.setOptionValue("output_flag", True); h.setOptionValue("log_to_console", False); h.setOptionValue("log_file", sys.argv[2])
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", 600.0); h.setOptionValue("mip_max_nodes", 1); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); t = time.perf_counter() - t
i = h.getInfo()
print("RES " + json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value, "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": t}))
"""
OCODE = """
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
mo, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = mo.solve(None, False, 600.0, None, 1, 4, True); t = time.perf_counter() - t
print("RES " + json.dumps({k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "lp_iterations", "root_bound")} | {"time": t}))
"""
for p in probs:
    mps = str(WT / ".miplib_cache" / f"{p}.mps")
    for who in (["highs", "ours"] if which == "both" else [which]):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        if who == "ours":
            env.update(m.CAND1118); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_MIP_LOG"] = "1"
            for x in kv:
                k, v = x.split("=", 1); env[k] = v
            lf = D / "logs" / f"ours{TAG}_{p}.log"
            with open(lf, "w") as f:
                pr = subprocess.run([sys.executable, "-c", OCODE, mps], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=f, text=True)
        else:
            lf = D / "logs" / f"highs_{p}.log"
            pr = subprocess.run([sys.executable, "-c", HCODE, mps, str(lf)], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        res = [l for l in pr.stdout.splitlines() if l.startswith("RES ")]
        r = json.loads(res[-1][4:]) if res else {"status": "crash"}
        row = {"problem": p, "who": who + TAG, "env": kv if who == "ours" else []} | r
        with open(D / "res.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)
