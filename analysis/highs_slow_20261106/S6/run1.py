"""S6 の 1 回の計測 (2026-11-06): 1 問を ENOMOTO (環境変数の組) か HiGHS で 4 スレッドで解き、結果を S6/runs.jsonl に、ログを S6/logs/ に書く。
使い方: python run1.py 名前 問題 秒 種 [KEY=VAL ...]   (名前 = highs なら HiGHS、ほかは ENOMOTO で KEY=VAL を環境変数に足す)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
name, p, tl, seed = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
kv = dict(a.split("=", 1) for a in sys.argv[5:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env["ENOMOTO_MIP_THREADS"] = "4"
kind = "highs" if name == "highs" else "ours"
if kind == "highs":
    env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
else:
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=seed, **kv)
(D / "logs").mkdir(exist_ok=True)
logf = D / "logs" / f"{p}_{name}_s{seed}_{tl}.log"
t0 = time.time(); peak = 0.0
with open(logf, "w") as lf:
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True)
    while pr.poll() is None:
        try:
            peak = max(peak, next(int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:")))
        except (OSError, StopIteration):
            pass
        time.sleep(1)
        if time.time() - t0 > tl + 300: pr.kill(); break
    out, _ = pr.communicate()
lines = out.splitlines()
if kind == "highs":
    with open(logf, "a") as lf: lf.write(out)
try:
    res = json.loads([l for l in lines if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
o, b = res.get("objective"), res.get("bound")
gap = abs(o - b) / max(abs(o), abs(b), 1e-9) if o is not None and b is not None else None
rec = {"problem": p, "name": name, "seed": seed, "tl": tl, "env": kv, "result": res, "gap": gap, "max_rss_mb": round(peak),
       "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}
with open(D / "runs.jsonl", "a") as f: f.write(json.dumps(rec) + "\n")
print(p, name, seed, res.get("status"), o, b, gap and round(gap, 5), round(res.get("time") or 0, 1), flush=True)
