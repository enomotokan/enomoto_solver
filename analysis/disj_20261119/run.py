"""1 回の計測 (2026-11-19、analysis/disj_20261119.md、redsplit_20261107/run.py を写したもの): ENOMOTO を解き、結果と
根のカットのループを抜けた理由 (診断 `ENOMOTO_MIP_CUTSTOP_LOG=1` の `CUTSTOP {...}` の行) を jsonl に足し、詳細ログを
logs/<問題>_<時間>_s<種>[_root].err.gz に置く。ENOMOTO_ の環境変数は消してから決めたものだけを与える (既定の動作)。
使い方: python run.py 問題 時間 種 出力 [root]  (root ならノード上限 1 で根だけ。種は ENOMOTO_T_MIP_SEED)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p, tl, seed, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
root = len(sys.argv) > 5 and sys.argv[5] == "root"
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_CUTSTOP_LOG="1")
if root:
    env["ENOMOTO_MIP_NODE_LIMIT"] = "1"
t0 = time.time()
try:
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    err = r.stderr
except Exception as e:  # noqa: BLE001
    res = {"status": "crash", "error": str(e)[:300]}
    err = getattr(e, "stderr", "") or ""
    if isinstance(err, bytes):
        err = err.decode(errors="replace")
stops = []
for l in err.splitlines():
    if l.startswith("CUTSTOP "):
        try:
            stops.append(json.loads(l[8:]))
        except ValueError:
            pass
# 前処理の後の問題の大きさ・根の LP の値 (ログにあれば)
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"{p}_{tl}_s{seed}{'_root' if root else ''}.err.gz", "wt") as f:
    f.write(err)
rec = {"problem": p, "tl": tl, "seed": seed, "root": root, "wall": round(time.time() - t0, 1), "result": res, "cutstop": stops,
       "warn": err.count("MIP: WARNING"), "at": time.strftime("%H:%M:%S")}
with open(D / out, "a") as f:
    f.write(json.dumps(rec) + "\n")
main = [s for s in stops if not s["bg"]]
print(p, seed, res.get("status"), res.get("time"), [(s["reason"], s["restarts"], s["rounds"], s["nfrac"]) for s in main], flush=True)
