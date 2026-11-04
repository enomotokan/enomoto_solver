"""mitt600 と同じ求解 (scripts/run_miplib_benchmark.py --worker-ours、ログなし、4 スレッド) を 1 回回し、2 秒ごとの RSS を取る (2026-11-04、memcap_c)。
使い方: python run_worker.py <名前> <問題> <時間の上限[秒]>  (切り替えの環境変数はそのまま子に渡す)。結果は worker_<名前>.jsonl に 1 行足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
name, prob, tl = sys.argv[1], sys.argv[2], sys.argv[3]
mps = WT / ".miplib_cache" / f"{prob}.mps"
env = dict(os.environ, ENOMOTO_MIP_THREADS="4")
t0 = time.time(); series = []; mx = 0
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
while pr.poll() is None:
    time.sleep(2)
    try:
        rss = [int(l.split()[1]) >> 10 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS")][0]
        series.append([round(time.time() - t0, 1), rss]); mx = max(mx, rss)
    except (OSError, IndexError):
        pass
out = pr.stdout.read()
js = [l for l in out.splitlines() if l.startswith("{")]
res = json.loads(js[-1]) if js else {"status": "crash", "rc": pr.returncode}
rec = {"name": name, "problem": prob, "time_limit": float(tl), "env": {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_")},
       "result": res, "rc": pr.returncode, "wall": round(time.time() - t0, 1), "max_rss_mb": mx, "series": series, "at": time.strftime("%Y-%m-%d %H:%M:%S")}
with open(D / f"worker_{name}.jsonl", "a") as f:
    f.write(json.dumps(rec) + "\n")
print(name, res.get("status"), res.get("objective"), res.get("bound"), mx, rec["wall"], flush=True)
