"""INCUMB: 1 回分を解く。使い方: python run1.py 設定 問題 種 [秒]
設定は CFGS の名前 (環境変数の組)。出力: res.jsonl (1 行 1 回)、logs/<設定>_<問題>_s<種>.log.gz (各行の頭に経過秒)。
ENOMOTO は 4 スレッド・ENOMOTO_MIP_LOG=1、HiGHS (設定 highs) は mip_rel_gap=0 (run_miplib_benchmark.py の worker)。ソルバーは同時に 1 つだけ回すこと。"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "highs": None,
    "r": {"ENOMOTO_MIP_INCUMB": "r"},
    "a": {"ENOMOTO_MIP_INCUMB": "a"},
    "ra": {"ENOMOTO_MIP_INCUMB": "ra"},
}
cfg, prob, seed = sys.argv[1], sys.argv[2], int(sys.argv[3])
TL = float(sys.argv[4]) if len(sys.argv) > 4 else 600
OUT = D / "res.jsonl"
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
solver = "highs" if cfg == "highs" else "ours"
if solver == "ours":
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", **CFGS[cfg])
else:
    env.update(ENOMOTO_BENCH_HIGHS_LOG="1")
mps = rb.fetch(prob, WT / ".miplib_cache")
log = D / "logs" / f"{cfg}_{prob}_s{seed}.log.gz"
t0 = time.time(); peak = 0.0; last = [""]
pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{solver}", str(mps), "--time-limit", str(TL), "--threads", "4"],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
def pump():
    with gzip.open(log, "wt") as g:
        for line in pr.stdout:
            if line.startswith("{"): last[0] = line
            g.write(f"{time.time() - t0:8.2f} {line}")
th = threading.Thread(target=pump); th.start()
while pr.poll() is None:
    peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
    if time.time() - t0 > TL + 300: pr.kill(); break
th.join()
try: res = json.loads(last[0])
except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
with open(OUT, "a") as f:
    f.write(json.dumps({"cfg": cfg, "problem": prob, "seed": seed, "tl": TL, "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1),
                        "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
print(cfg, prob, seed, res.get("status"), res.get("time"), res.get("objective"), round(peak), flush=True)
