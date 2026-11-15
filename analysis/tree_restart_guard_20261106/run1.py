"""木の中の再スタートの見分けの計測 (2026-11-06)。1 回を 4 スレッドで解き、結果を res.jsonl に足し、ENOMOTO のログ
(ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_TREE_RESTART_LOG=1 の stderr) を logs/<タグ>_<問題>_<種>.log に置く。
使い方: python analysis/tree_restart_guard_20261106/run1.py 問題 種 タグ [名前=値 ...]  (RUN_TL で時間の上限、既定 60 秒)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = int(os.environ.get("RUN_TL", "60"))
p, seed, tag = sys.argv[1:4]
extra = dict(kv.split("=", 1) for kv in sys.argv[4:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TREE_RESTART_LOG="1", ENOMOTO_T_MIP_SEED=seed, **extra)
(D / "logs").mkdir(parents=True, exist_ok=True)
logf = D / f"logs/{tag}_{p}_{seed}.log"
t0 = time.time()
pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                    cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
logf.write_text(pr.stderr)
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "seed": seed, "tag": tag, "tl": TL, "env": extra, "result": res, "wall": round(time.time() - t0, 1),
                        "at": time.strftime("%H:%M:%S")}) + "\n")
print(tag, p, seed, res.get("status"), round(res.get("time") or 0, 1), res.get("nodes"), flush=True)
