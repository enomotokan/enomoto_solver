"""1 回だけ解いてログを残す (analysis/par4slow_20261119/one.py の写しに ENOMOTO_PROF_TREE_PAR=1 を足した): python one.py 問題 スレッド数 種 秒 札 [ENV=値 ...]。
ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_TREE_DIAG=1・ENOMOTO_PROF_TREE_PAR=1 で標準エラーを経過秒付きで logs/log_問題_札.log.gz に、結果の JSON と最大 RSS を logs/log_問題_札.out に書く。"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p, th, seed, tl, tag = sys.argv[1:6]
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": seed, "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_TREE_DIAG": "1", "ENOMOTO_PROF_TREE_PAR": "1"})
for kv in sys.argv[6:]:
    k, v = kv.split("=", 1); env[k] = v
t0 = time.time()
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
lg = gzip.open(D / "logs" / f"log_{p}_{tag}.log.gz", "wt")
def pump():
    for l in pr.stderr: lg.write(f"{time.time() - t0:.2f}\t{l}")
tp = threading.Thread(target=pump); tp.start()
peak = 0.0
while pr.poll() is None:
    try: peak = max(peak, [int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:")][0])
    except Exception: pass  # noqa: BLE001
    time.sleep(0.5)
out = pr.stdout.read(); tp.join(); lg.close()
res = [l for l in out.splitlines() if l.startswith("{")]
line = json.dumps({"problem": p, "tag": tag, "threads": th, "seed": seed, "env": sys.argv[6:], "result": json.loads(res[-1]) if res else "crash",
                   "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1)})
(D / "logs" / f"log_{p}_{tag}.out").write_text(line + "\n"); print(line)
