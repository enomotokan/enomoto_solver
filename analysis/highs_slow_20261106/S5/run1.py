"""S5 の計測 (2026-11-06): 1 問を ENOMOTO (環境変数を指定) か HiGHS で 4 スレッドで 1 回解き、結果を S5/res.jsonl に、ログを S5/logs/ に書く。
使い方: python run1.py ours|highs 問題 名札 時間 [KEY=VAL ...]   (ours は ENOMOTO_MIP_LOG=1、HiGHS は ENOMOTO_BENCH_HIGHS_LOG=1)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
kind, p, tag, tl = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
extra = dict(kv.split("=", 1) for kv in sys.argv[5:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_BENCH_HIGHS_LOG="1", **extra)
(D / "logs").mkdir(exist_ok=True)
logf = D / "logs" / f"{p}_{kind}_{tag}.log"
t0 = time.time()
with open(logf, "w") as lf:
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True, timeout=tl + 300)
lf = open(logf, "a"); lf.write(pr.stdout); lf.close()
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "solver": kind, "tag": tag, "env": extra, "result": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
print(p, kind, tag, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
