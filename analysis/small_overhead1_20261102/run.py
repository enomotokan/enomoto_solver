"""small_overhead1 (2026-11-02): 担当の小さい問題を ENOMOTO (4 スレッド・60 秒、ENOMOTO_MIP_LOG=1) と HiGHS で解き、ログを logs/ に残す。
使い方: python run.py <問題,...> <設定名> <回数> [KEY=VAL ...]   設定名 highs は HiGHS (ログつき、--worker-highs と同じ設定)。
結果は results.jsonl に足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
HIGHS = r'''
import highspy, json, sys, time
h = highspy.Highs(); h.setOptionValue("time_limit", 60.0); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); t = time.perf_counter() - t; i = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value, "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": t}))
'''
probs, cfg, n = sys.argv[1].split(","), sys.argv[2], int(sys.argv[3])
extra = dict(a.split("=", 1) for a in sys.argv[4:])
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for k in range(n):
        env = {k2: v for k2, v in os.environ.items() if not k2.startswith("ENOMOTO_")}
        if cfg == "highs":
            cmd = [sys.executable, "-c", HIGHS, str(mps)]
        else:
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **extra)
            cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"]
        t0 = time.perf_counter()
        r = subprocess.run(cmd, cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        wall = time.perf_counter() - t0
        log = r.stderr + r.stdout
        (D / "logs" / f"{p}_{cfg}_{k}.log").write_text(log)
        try:
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception:  # noqa: BLE001
            res = {"status": "crash"}
        with open(D / "results.jsonl", "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "run": k, "env": extra, "wall": wall, "result": res}) + "\n")
        print(f"{p:14s} {cfg:10s} {k} {res.get('status'):10s} t={res.get('time', 0):6.2f}s wall={wall:5.2f} obj={res.get('objective')}", flush=True)
