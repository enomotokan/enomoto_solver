"""S2 組 (h80x6320d・drayage-25-23・neos-2978193-inde) の計測 (2026-11-06)。
1 回を 4 スレッドで解き、結果を S2/<タグ>.jsonl に足し、ENOMOTO のログ (ENOMOTO_MIP_LOG=1 の stderr) を S2/logs/<タグ>_<問題>_<種>.log に置く。
使い方: python analysis/highs_slow_20261106/run_s2.py ours|highs 問題 種 タグ [名前=値 ...]
  (名前=値 は ENOMOTO の環境変数。既定の回は何も付けない。RUN_TL で時間の上限、既定 600 秒)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = int(os.environ.get("RUN_TL", "600"))
kind, p, seed, tag = sys.argv[1:5]
extra = dict(kv.split("=", 1) for kv in sys.argv[5:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_") or k.startswith("ENOMOTO_BENCH_HIGHS_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=seed, **extra)
if kind == "highs":
    env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
(D / "S2/logs").mkdir(parents=True, exist_ok=True)
logf = D / f"S2/logs/{tag}_{p}_{seed}.log"
t0 = time.time()
pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(TL), "--threads", "4"],
                    cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
logf.write_text(pr.stderr + ("\n# stdout\n" + pr.stdout if kind == "highs" else ""))
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / f"S2/{tag}.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "solver": kind, "seed": seed, "env": extra, "result": res, "wall": round(time.time() - t0, 1),
                        "at": time.strftime("%H:%M:%S")}) + "\n")
print(tag, p, seed, res.get("status"), round(res.get("time") or 0, 1), res.get("nodes"), flush=True)
