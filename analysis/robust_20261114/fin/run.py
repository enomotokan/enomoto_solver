"""仕上げの対の分担を回す: python run.py K (shards.json の K 番目を順に 4 スレッド・60 秒で解き、res_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ)。
組は cand1114 (束そのもの) と pgq (cand1114 + ENOMOTO_MIP_ROBUST=pgq)。元の MPS (並べ替えなし) を種 ENOMOTO_T_MIP_SEED で解く。
ログ (ENOMOTO_MIP_LOG=1、表示だけ) は logs/<組>/<問題>_s<種>.log.gz。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113"))
from probs import rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
CFGS = {"cand1114": {}, "pgq": {"ENOMOTO_MIP_ROBUST": "pgq"}}
K = int(sys.argv[1]); TL = 60.0; TH = 4
mine = json.loads((D / "shards.json").read_text())["shards"][K]
OUT = D / f"res_s{K}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["seed"], r["cfg"]))
for p, s, cfg in mine:
    if (p, s, cfg) in done: continue
    for attempt in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception:  # noqa: BLE001
            if attempt == 4: raise
            time.sleep(30)
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CAND1114); env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = str(TH); env["ENOMOTO_T_MIP_SEED"] = str(s); env["ENOMOTO_MIP_LOG"] = "1"
    logd = D / "logs" / cfg; logd.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    with gzip.open(logd / f"{p}_s{s}.log.gz", "wt") as lf:
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", str(TH)],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        out, err = pr.communicate(timeout=TL + 600)
        lf.write(err)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    row = {"problem": p, "threads": TH, "perm": 0, "seed": s, "tl": TL, "result": res, "wall": round(time.time() - t0, 1),
           "cfg": cfg, "shard": K, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, s, cfg, res.get("status"), res.get("time"), flush=True)
