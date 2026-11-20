"""擬似費用の予測と実際の上がり幅の計測 (2026-11-20、analysis/disj_20261119.md の 11 節)。ENOMOTO を解き、結果を jsonl に足し、
診断 `ENOMOTO_MIP_PCLOG=1` の `PCO` の行を logs/pc_<設定>_<問題>_<時間>_s<種>.pco.gz に、それ以外のログを .err.gz に置く。
使い方: python run_pc.py 問題 時間 種 出力  (設定は環境変数 CFG、既定 def。PCLOG=0 なら診断を出さない)"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
# 11 節の試作 `ENOMOTO_MIP_PCL` (文字 1 つずつ)
CFGS = {"def": {}, "a": {"ENOMOTO_MIP_PCL": "a"}, "b": {"ENOMOTO_MIP_PCL": "b"}, "c": {"ENOMOTO_MIP_PCL": "c"}, "d": {"ENOMOTO_MIP_PCL": "d"}}
p, tl, seed, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
cfg = os.environ.get("CFG", "def")
pclog = os.environ.get("PCLOG", "1")
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_PCLOG=pclog, **CFGS[cfg])
t0 = time.time()
(D / "logs").mkdir(exist_ok=True)
base = D / "logs" / f"pc_{cfg}_{p}_{tl}_s{seed}"
with open(str(base) + ".raw", "w") as ef:
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                           cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True, timeout=tl + 400)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "error": str(e)[:300]}
npco = 0
with open(str(base) + ".raw") as f, gzip.open(str(base) + ".pco.gz", "wt") as fp, gzip.open(str(base) + ".err.gz", "wt") as fe:
    for l in f:
        if l.startswith("PCO "):
            fp.write(l[4:]); npco += 1
        else:
            fe.write(l)
os.remove(str(base) + ".raw")
rec = {"problem": p, "cfg": cfg, "tl": tl, "seed": seed, "pclog": pclog, "wall": round(time.time() - t0, 1), "result": res, "npco": npco, "at": time.strftime("%H:%M:%S")}
with open(D / out, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, cfg, seed, res.get("status"), res.get("time"), res.get("nodes"), npco, flush=True)
