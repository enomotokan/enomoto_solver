"""大きな問題の def と dc (`ENOMOTO_MIP_DSE_COST=1`) の比べ: python run_big.py 出力名 問題... (4 スレッド・種 1・600 秒、1 本ずつ)。
各回の stderr に経過秒を付けて logs/<問題>_<設定>.log、結果は <出力名>.jsonl に足す (途中から再開できる)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]; L = D / "logs"
out = D / f"{sys.argv[1]}.jsonl"; probs = sys.argv[2:]
CF = {"def": {}, "dc": {"ENOMOTO_MIP_DSE_COST": "1"}}
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(out)} if out.exists() else set()
for p in probs:
    for c, extra in CF.items():
        if (p, c) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", **extra)
        t0 = time.time()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "600", "--threads", "4"],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        keep = []
        with open(L / f"{p}_{c}.log", "w") as f:
            for l in pr.stderr:
                f.write(f"[{time.time() - t0:8.2f}] {l}")
        so = pr.stdout.read(); pr.wait()
        try: res = json.loads([l for l in so.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        with open(out, "a") as f: f.write(json.dumps({"problem": p, "cfg": c, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), flush=True)
