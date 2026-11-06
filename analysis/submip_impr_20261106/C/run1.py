"""担当 C の計測 (2026-11-06。A/run1.py の写し): 1 問を ENOMOTO (既定 + 試作の環境変数) か HiGHS で 4 スレッドで 1 回解き、res.jsonl に足す。
ENOMOTO は ENOMOTO_MIP_LOG=1 で、ログ (stderr) を logs/<タグ>_<問題>_s<種>.log に残す。HiGHS は ENOMOTO_BENCH_HIGHS_LOG=1 でログを残す。
使い方: python run1.py ours|highs 問題 タグ 種 [時間の上限] [KEY=VAL ...]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
kind, p, tag, seed = sys.argv[1:5]
tl = int(sys.argv[5]) if len(sys.argv) > 5 else 600
extra = dict(a.split("=", 1) for a in sys.argv[6:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env["ENOMOTO_MIP_THREADS"] = "4"
if kind == "ours":
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=seed, **extra)
else:
    env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
t0 = time.time()
pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                    cwd=WT, env=env, capture_output=True, text=True)
log = D / "logs" / f"{tag}_{p}_s{seed}.log"
log.write_text(pr.stderr + ("\n=== stdout ===\n" + pr.stdout if kind == "highs" else ""))
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "solver": kind, "tag": tag, "seed": seed, "env": extra, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
print(p, kind, tag, seed, res.get("status"), round(res.get("time") or 0, 1), res.get("objective"), res.get("nodes"), flush=True)
