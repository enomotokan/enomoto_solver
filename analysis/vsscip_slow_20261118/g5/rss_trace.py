"""RSS の時間の並びを取る: python rss_trace.py 問題 設定 種 秒 [スレッド数] (設定は cfg.py、スレッド数の既定は 4)。
0.5 秒おきの RSS を rsst_問題_設定_s種.tsv (経過秒・RSS MB) に、標準エラー (ENOMOTO_MIP_TREE_DIAG=1) を経過秒を頭に付けて rsst_問題_設定_s種.log.gz に書く。"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
p, cfg, seed, tl = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
th = sys.argv[5] if len(sys.argv) > 5 else "4"
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CFGS[cfg]); env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": seed, "ENOMOTO_MIP_TREE_DIAG": "1"})
tag = f"rsst_{p}_{cfg}_s{seed}" + ("" if th == "4" else f"_t{th}")
t0 = time.time()
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
lg = gzip.open(D / f"{tag}.log.gz", "wt")
def pump():
    for l in pr.stderr: lg.write(f"{time.time() - t0:.2f}\t{l}")
tp = threading.Thread(target=pump); tp.start()
with open(D / f"{tag}.tsv", "w") as f:
    while pr.poll() is None:
        try: r = [int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:")][0]
        except Exception: r = 0.0  # noqa: BLE001
        f.write(f"{time.time() - t0:.2f}\t{r:.0f}\n"); time.sleep(0.5)
out = pr.stdout.read(); tp.join(); lg.close()
res = [l for l in out.splitlines() if l.startswith("{")]
print(tag, res[-1] if res else "crash")
