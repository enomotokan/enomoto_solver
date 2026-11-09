"""GAP2 の計測 (2026-11-09): 1 問を ENOMOTO か HiGHS で 4 スレッドで 1 回解く。
使い方: python run1.py ours|highs 問題 名札 種 時間 [KEY=VAL ...]
ours は ENOMOTO_MIP_LOG=1・ENOMOTO_T_MIP_SEED=種、highs は ENOMOTO_BENCH_HIGHS_LOG=1・random_seed=種。
ログは書き込み中に rebase --autostash で壊れないよう /home/user/gap2work/logs/ に書き、コミットの前に logs/ へ写す。結果は res.jsonl に足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
W = Path("/home/user/gap2work")
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
kind, p, tag, seed, tl = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
extra = dict(kv.split("=", 1) for kv in sys.argv[6:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_BENCH_HIGHS_LOG="1", ENOMOTO_T_MIP_SEED=seed, **extra)
if kind == "highs":
    env["ENOMOTO_BENCH_HIGHS_OPT"] = ",".join(filter(None, [env.get("ENOMOTO_BENCH_HIGHS_OPT", ""), f"random_seed={seed}"]))
(W / "logs").mkdir(parents=True, exist_ok=True)
logf = W / "logs" / f"{p}_{kind}_{tag}_s{seed}.log"
t0 = time.time()
with open(logf, "w") as lf:
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True, timeout=tl + 300)
with open(logf, "a") as lf:
    lf.write(pr.stdout)
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(W / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "solver": kind, "tag": tag, "seed": seed, "env": extra, "result": res,
                        "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
print(p, kind, tag, seed, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
