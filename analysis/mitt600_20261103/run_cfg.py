"""600 秒の比べの追加 (2026-11-04): 1 問を ENOMOTO の指定の設定 (analysis/halve_submip_20261030/run.py の CFGS) で 4 スレッド・600 秒で解き、
cfg_<設定>_<問題>.jsonl に書く。最大 RSS は 1 秒ごとの VmRSS、ほかは run600.py と同じ。
使い方: python analysis/mitt600_20261103/run_cfg.py 設定 問題"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis/halve_submip_20261030"))
import run_miplib_benchmark as rb  # noqa: E402
from run import CFGS  # noqa: E402
TL = 600
cfg, p = sys.argv[1], sys.argv[2]
OUT = D / f"cfg_{cfg}_{p}.jsonl"
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", **CFGS[cfg])
t0 = time.time(); peak = 0.0; series = []
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
while pr.poll() is None:
    try:
        rss = next(int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:"))
    except (OSError, StopIteration):
        rss = 0.0
    peak = max(peak, rss)
    el = time.time() - t0
    if not series or el - series[-1][0] >= 30: series.append((round(el), round(rss)))
    time.sleep(1)
    if el > TL + 300: pr.kill(); break
out, err = pr.communicate()
try:
    res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode, "error": err[-300:]}
with open(OUT, "a") as f:
    f.write(json.dumps({"problem": p, "solver": "ours", "cfg": cfg, "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1),
                        "rss_series": series, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
print(p, cfg, res.get("status"), res.get("time"), round(peak), flush=True)
