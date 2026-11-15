"""small_overhead2 (2026-11-02): 担当 4 問を ENOMOTO (4 スレッド・60 秒、ENOMOTO_MIP_LOG=1) と HiGHS (ログ付き) で解く。
使い方: python run.py <問題,...> <回数> <タグ> [KEY=VAL ...] (タグ highs なら HiGHS)。ソルバーは同時に 1 つ。"""
import json, os, subprocess, sys, threading, time
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
probs, n, tag = sys.argv[1].split(","), int(sys.argv[2]), sys.argv[3]
extra = dict(a.split("=", 1) for a in sys.argv[4:])
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for k in range(n):
        env = {k2: v for k2, v in os.environ.items() if not k2.startswith("ENOMOTO_")}
        if tag == "highs":
            cmd = [sys.executable, "-c", HIGHS, str(mps)]
        else:
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **extra)
            cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"]
        # 標準エラー (開発用のログ) は 1 行ずつ、起動からの経過秒を頭に付けて残す
        t0 = time.perf_counter()
        pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
        errs = []
        th = threading.Thread(target=lambda: [errs.append(f"[{time.perf_counter() - t0:7.3f}] {l}") for l in pr.stderr])
        th.start(); out = pr.stdout.read(); pr.wait(timeout=300); th.join()
        r = subprocess.CompletedProcess(cmd, pr.returncode, out, "".join(errs))
        log = r.stderr + r.stdout
        (D / "logs" / f"{p}_{tag}_{k}.log").write_text(log)
        try:
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "time": -1, "error": log[-300:]}
        with open(D / "results.jsonl", "a") as f:
            f.write(json.dumps({"problem": p, "tag": tag, "rep": k, "env": extra, "result": res}) + "\n")
        print(f"{p:16s} {tag:10s} {k} {res['status']:12s} {res['time']:6.2f}s obj={res.get('objective')} nodes={res.get('nodes')}", flush=True)
