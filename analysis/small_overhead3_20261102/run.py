"""small_overhead3 (2026-11-02): 担当の問題を ENOMOTO (4 スレッド・60 秒、ENOMOTO_MIP_LOG=1) と HiGHS (ログあり) で解く。
使い方: python run.py <tag> <問題,...> <回数> [KEY=VAL ...]   (tag=highs で HiGHS)。結果は res.jsonl に足し、ログは logs/。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
HIGHS = r'''
import sys, time, json, highspy
h = highspy.Highs(); h.setOptionValue("time_limit", 60.0); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); t = time.perf_counter() - t; i = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value, "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": t}))
'''
tag, probs, reps = sys.argv[1], sys.argv[2].split(","), int(sys.argv[3])
extra = dict(a.split("=", 1) for a in sys.argv[4:])
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for k in range(reps):
        env = {k2: v for k2, v in os.environ.items() if not k2.startswith("ENOMOTO_")}
        if tag == "highs":
            cmd = [sys.executable, "-c", HIGHS, str(mps)]
        else:
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **extra)
            cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"]
        r = subprocess.run(cmd, cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        (D / "logs" / f"{p}_{tag}_{k}.log").write_text(r.stderr + r.stdout)
        try:
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": (r.stderr or str(e))[-300:], "time": 999}
        with open(D / "res.jsonl", "a") as f:
            f.write(json.dumps({"problem": p, "tag": tag, "rep": k, "env": extra, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(f"{p:20s} {tag:12s} {k} {res['status']:10s} {res['time']:6.2f}s obj={res.get('objective')} nodes={res.get('nodes')}", flush=True)
