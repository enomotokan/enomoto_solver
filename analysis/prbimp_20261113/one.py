"""PRBIMP: 1 回解いてログを残す。python one.py 設定 問題 種 秒 [環境変数=値 ...]
設定は cfg.py の CFGS の名前 (cand1114 の上に足す形)。ログは logs/<問題>.<設定>.s<種>.log.gz (ENOMOTO_MIP_LOG=1)、結果を res_one.jsonl に足す。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
name, p, seed, tl = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
extra = dict(kv.split("=", 1) for kv in sys.argv[5:])
tag = name + ("+" + "+".join(f"{k.replace('ENOMOTO_', '')}={v}" for k, v in extra.items()) if extra else "")
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CFGS[name]); env.update(extra)
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
(D / "logs").mkdir(exist_ok=True)
t0 = time.time()
pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
res = {"status": "crash"}
lname = D / "logs" / f"{p}.{tag}.s{seed}.log.gz"
with gzip.open(lname, "wt") as f:
    for line in pr.stdout:
        f.write(f"{time.time() - t0:8.2f} {line}")
        if line.startswith("{"):
            try: res = json.loads(line)
            except Exception: pass  # noqa: BLE001
pr.wait()
row = {"problem": p, "cfg": tag, "seed": seed, "tl": tl, "result": res, "wall": round(time.time() - t0, 1)}
with open(D / "res_one.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
print(p, tag, seed, res.get("status"), res.get("time"), res.get("objective"), flush=True)
