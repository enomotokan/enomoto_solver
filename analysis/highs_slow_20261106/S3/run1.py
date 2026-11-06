"""S3 (supportcase33・neos-1445765・tbfp-network) の 1 回の計測 (2026-11-06)。
使い方: python run1.py ours|highs 問題 タグ 種 [時間の上限] [名前=値 ...]
ENOMOTO は ENOMOTO_MIP_LOG=1・ENOMOTO_T_MIP_SEED=種、HiGHS はログ付き・random_seed=種。4 スレッド。
ログを logs/<問題>_<タグ>_<種>.log、結果を res.jsonl に足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
kind, p, tag, seed = sys.argv[1:5]
TL = int(sys.argv[5]) if len(sys.argv) > 5 else 600
extra = dict(a.split("=", 1) for a in sys.argv[6:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env["ENOMOTO_MIP_THREADS"] = "4"
if kind == "ours":
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=seed)
else:
    env.update(ENOMOTO_BENCH_HIGHS_LOG="1", ENOMOTO_BENCH_HIGHS_OPT=f"random_seed={seed}")
env.update(extra)
log = D / "logs" / f"{p}_{tag}_{seed}.log"
# 走っている間は作業ツリーの外に書き、終わってから logs/ へ移す (git の rebase で書いている途中のファイルが作り直されないように)
live = Path("/tmp/s3_live") / log.name
live.parent.mkdir(exist_ok=True)
t0 = time.time()
with open(live, "w") as lf:
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(TL),
                         "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True, timeout=TL + 300)
out = pr.stdout
with open(live, "a") as lf:
    lf.write(out)
import shutil  # noqa: E402
shutil.move(str(live), str(log))
try:
    res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "solver": kind, "tag": tag, "seed": seed, "tl": TL, "env": extra, "result": res,
                        "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
print(p, kind, tag, seed, res.get("status"), res.get("time"), res.get("objective"), flush=True)
