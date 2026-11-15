"""(2026-11-08: 回の番号を ENOMOTO_T_MIP_SEED にも使う) 1 問を 4 スレッドで解き、結果を <出力>.jsonl に足す。ログ (ENOMOTO_MIP_LOG=1) は logs/ に置く。
使い方: python run1.py 設定の名前 問題 時間 回の番号 出力の名前 [KEY=VAL ...]
設定の名前が CFGS にあればその環境変数、なければ引数の KEY=VAL だけを使う。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
CFGS = {"def": {}}
cfg, p, tl, rep, out = sys.argv[1:6]
extra = dict(a.split("=", 1) for a in sys.argv[6:])
mps = WT / ".miplib_cache" / f"{p}.mps"
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=rep, **CFGS.get(cfg, {}), **extra)
(D / "logs").mkdir(exist_ok=True)
log = D / "logs" / f"{out}_{cfg}_{p}_{rep}.log"
t0 = time.time()
pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"],
                    cwd=WT, env=env, stdout=subprocess.PIPE, stderr=open(log, "w"), text=True, timeout=float(tl) + 300)
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
with open(D / f"{out}.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "cfg": cfg, "rep": int(rep), "env": extra, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("best_bound"), flush=True)
