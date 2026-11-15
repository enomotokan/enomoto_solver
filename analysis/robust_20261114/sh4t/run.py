"""4 スレッドの 2 組の分担を回す: python run.py K [名前] (shards[_名前].json の K 番目を順に 4 スレッドで解き、res[_名前]_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ)。
行の形は ../run.py と同じ (problem, threads, perm, seed, cfg, result, ...)。並べ替え k ≥ 1 は ../perm.py で .miplib_cache/perm/<問題>_p<k>.mps を作る
(種 k の乱数で決まる)。ログ (ENOMOTO_MIP_LOG=1、表示だけ) は logs/[<名前>_]<組>/<問題>_t4_p<k>s<種>.log.gz。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113")); sys.path.insert(0, str(D.parent))
from probs import rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
import perm as pm  # noqa: E402
from cfgs import CFGS  # noqa: E402
K = int(sys.argv[1]); TL = 60.0; TH = 4
TAG = sys.argv[2] if len(sys.argv) > 2 else ""
mine = json.loads((D / (f"shards_{TAG}.json" if TAG else "shards.json")).read_text())["shards"][K]
OUT = D / (f"res_{TAG}_s{K}.jsonl" if TAG else f"res_s{K}.jsonl")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["perm"], r["seed"], r["cfg"]))
def mps_of(p, k):
    for attempt in range(5):
        try: src = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception:  # noqa: BLE001
            if attempt == 4: raise
            time.sleep(30)
    if k == 0: return src
    out = WT / ".miplib_cache/perm" / f"{p}_p{k}.mps"
    if not out.exists(): out.parent.mkdir(parents=True, exist_ok=True); pm.make(src, k, out)
    return out
for p, k, s, cfg in mine:
    if (p, k, s, cfg) in done: continue
    mps = mps_of(p, k)
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CAND1114); env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = str(TH); env["ENOMOTO_T_MIP_SEED"] = str(s); env["ENOMOTO_MIP_LOG"] = "1"
    logd = D / "logs" / (f"{TAG}_{cfg}" if TAG else cfg); logd.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    with gzip.open(logd / f"{p}_t{TH}_p{k}s{s}.log.gz", "wt") as lf:
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", str(TH)],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        out, err = pr.communicate(timeout=TL + 600)
        lf.write(err)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    row = {"problem": p, "threads": TH, "perm": k, "seed": s, "tl": TL, "result": res, "wall": round(time.time() - t0, 1),
           "cfg": cfg, "shard": K, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, k, s, cfg, res.get("status"), res.get("time"), flush=True)
