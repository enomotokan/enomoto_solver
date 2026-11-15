"""SMALLFIX: ENOMOTO (cand1114 = analysis/cand1114_20261113/cfg.py の CAND1114) に環境変数を足して 1 問を解く。
python eno.py 問題 名札 時間 スレッド [名前=値 ...]   ログは logs/eno_<問題>_<名札>.log.gz (ENOMOTO_MIP_LOG=1)、結果は eno.jsonl に 1 行。
種は ENOMOTO_T_MIP_SEED=1。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cand1114_20261113"))
from cfg import CAND1114  # noqa: E402
p, tag, tl, th = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
extra = dict(kv.split("=", 1) for kv in sys.argv[5:])
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CAND1114, ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", **extra)
t0 = time.time()
r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                    "--time-limit", tl, "--threads", th], cwd=WT, env=env, capture_output=True, text=True)
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"eno_{p}_{tag}.log.gz", "wt") as f:
    f.write(r.stderr)
try:
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "rc": r.returncode}
with open(D / "eno.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "tag": tag, "tl": float(tl), "threads": th, "extra": extra, "result": res, "wall": round(time.time() - t0, 1),
                        "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
print(p, tag, res.get("status"), res.get("objective"), res.get("bound"), res.get("nodes"), res.get("time"))
