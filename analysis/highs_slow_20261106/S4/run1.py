"""S4 の 1 回の計測 (2026-11-06): python run1.py ラベル 問題 {ours|highs} 種 [KEY=VAL ...]
ENOMOTO は ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_THREADS=4・ENOMOTO_T_MIP_SEED=種 (+ 指定の環境変数)、HiGHS は run600.py と同じ回し方 (ENOMOTO_BENCH_HIGHS_LOG=1 でログ付き)。
ログは log/<問題>_<ラベル>.txt、結果は res.jsonl に足す。時間の上限は RUN_TL (既定 600)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = int(os.environ.get("RUN_TL", "600"))
lab, p, kind, seed = sys.argv[1:5]
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4")
if kind == "ours":
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=seed)
else:
    env.update(ENOMOTO_BENCH_HIGHS_LOG="1")
extra = dict(a.split("=", 1) for a in sys.argv[5:])
env.update(extra)
(D / "log").mkdir(exist_ok=True)
logf = D / "log" / f"{p}_{lab}.txt"
t0 = time.time()
with open(logf, "w") as lf:
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(TL), "--threads", "4"],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=TL + 300)
    lf.write(pr.stdout)
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "label": lab, "solver": kind, "seed": seed, "env": extra, "tl": TL, "result": res,
                        "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
print(p, lab, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
