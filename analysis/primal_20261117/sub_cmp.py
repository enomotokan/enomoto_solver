"""調べ C の 8 節: 書き出した根の RENS の部分問題 (ENOMOTO_MIP_SUBMIP_DUMP) を、HiGHS 1.15.1 (1 スレッド、サブ MIP と同じ)・当方のサブ MIP の形
(ENOMOTO_MIP_DEBUG_AS_SUBMIP: 入れ子の深さ 1・1 スレッド)・当方の通常の形 (4 スレッド) で単独に 300 秒解き比べる。打ち切り値は与えない。
python sub_cmp.py 名前=MPS ...  (H115 に highspy 1.15.1 の python)。ログは logs/sub_<who>_<名前>.log、結果は sub_res.jsonl"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
H115 = os.environ.get("H115", sys.executable)
HCODE = """
import json, sys, time, highspy
h = highspy.Highs()
h.setOptionValue("output_flag", True); h.setOptionValue("log_to_console", False); h.setOptionValue("log_file", sys.argv[2])
h.setOptionValue("threads", 1); h.setOptionValue("time_limit", 300.0); h.setOptionValue("mip_rel_gap", 1e-4)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); t = time.perf_counter() - t
i = h.getInfo()
print("RES " + json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value, "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": t}))
"""
for arg in sys.argv[1:]:
    name, mps = arg.split("=", 1)
    for who in ("highs", "ours_sub", "ours_norm"):
        lf = D / "logs" / f"sub_{who}_{name}.log"
        if who == "highs":
            pr = subprocess.run([H115, "-c", HCODE, mps, str(lf)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            res = [l for l in pr.stdout.splitlines() if l.startswith("RES ")]
            r = json.loads(res[-1][4:]) if res else {"status": "crash"}
        else:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            th = "1" if who == "ours_sub" else "4"
            env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_MIP_LOG": "1"})
            if who == "ours_sub": env["ENOMOTO_MIP_DEBUG_AS_SUBMIP"] = "1"
            t0 = time.time()
            pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", mps, "--time-limit", "300", "--threads", th],
                                  cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
            out = []
            th_ = __import__("threading").Thread(target=lambda: out.append(pr.stdout.read())); th_.start()
            with open(lf, "w") as f:
                for line in pr.stderr:
                    f.write(f"[{time.time() - t0:8.2f}] {line}")
            pr.wait(); th_.join()
            try: r = json.loads([l for l in out[0].splitlines() if l.startswith("{")][-1])
            except Exception: r = {"status": "crash"}  # noqa: BLE001
        row = {"name": name, "who": who} | {k: r.get(k) for k in ("status", "objective", "bound", "best_bound", "nodes", "time")}
        with open(D / "sub_res.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)
