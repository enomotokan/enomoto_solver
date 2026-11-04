"""pool_bound2 の単発の求解 (2026-11-04)。scripts/run_miplib_benchmark.py --worker-ours を 4 スレッドで 1 回回し、2 秒ごとの RSS を取る
(memcap_c_20261104/run_worker.py の写しに、stderr を logs/<名前>_<問題>.log.gz に残すのを足した)。
使い方: python run.py <名前> <問題> <時間の上限[秒]>  (切り替えの環境変数はそのまま子に渡す。ENOMOTO_MIP_LOG=1 などは呼ぶ側で付ける)。
結果は runs.jsonl に 1 行足す。"""
import gzip, json, os, shutil, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
name, prob, tl = sys.argv[1], sys.argv[2], sys.argv[3]
mps = WT / ".miplib_cache" / f"{prob}.mps"
env = dict(os.environ, ENOMOTO_MIP_THREADS="4")
tmp = Path(f"/tmp/claude-0/pool2_{name}_{prob}.err")
t0 = time.time(); series = []; mx = 0
with open(tmp, "w") as ef:
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True)
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
with open(tmp, "rb") as src, gzip.open(D / "logs" / f"{name}_{prob}.log.gz", "wb") as dst:
    shutil.copyfileobj(src, dst)
tmp.unlink()
rec = {"name": name, "problem": prob, "time_limit": float(tl), "env": {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_")},
       "result": res, "rc": pr.returncode, "wall": round(time.time() - t0, 1), "max_rss_mb": mx, "series": series[::5], "at": time.strftime("%Y-%m-%d %H:%M:%S")}
with open(D / "runs.jsonl", "a") as f:
    f.write(json.dumps(rec) + "\n")
print(name, prob, res.get("status"), res.get("objective"), res.get("bound"), res.get("nodes"), mx, rec["wall"], flush=True)
