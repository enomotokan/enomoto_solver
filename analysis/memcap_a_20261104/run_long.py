"""memcap_a (2026-11-04): 1 問を設定 (下の CFGS) で 4 スレッド・TL 秒解き、long.jsonl に足す。1 秒ごとに VmRSS を読み (最大 RSS)、
10 秒ごとに推移を残す。標準エラーは logs/<問題>_<設定>_<TL>.err に書き、`MIP: MEMCAP:` の行を結果に入れる。
使い方: python analysis/memcap_a_20261104/run_long.py 設定 問題 TL"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "memcap": {"ENOMOTO_MIP_MEMCAP": "1"}}
cfg, p, TL = sys.argv[1], sys.argv[2], int(sys.argv[3])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", **CFGS[cfg])
(D / "logs").mkdir(exist_ok=True)
errp = D / "logs" / f"{p}_{cfg}_{TL}.err"
t0 = time.time(); peak = 0.0; series = []
with open(errp, "w") as ef:
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True)
    while pr.poll() is None:
        try:
            rss = next(int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:"))
        except (OSError, StopIteration):
            rss = 0.0
        peak = max(peak, rss)
        el = time.time() - t0
        if not series or el - series[-1][0] >= 10: series.append((round(el), round(rss)))
        time.sleep(1)
        if el > TL + 300: pr.kill(); break
    out, _ = pr.communicate()
err = errp.read_text(errors="replace")
try:
    res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode, "error": err[-300:]}
mc = [l for l in err.splitlines() if "MEMCAP" in l]
with open(D / "long.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "cfg": cfg, "tl": TL, "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1),
                        "rss_series": series, "memcap": mc, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
print(p, cfg, TL, res.get("status"), res.get("time"), round(peak), flush=True)
