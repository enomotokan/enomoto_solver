"""共有の診断の分担を回す: python run.py SET K (SET は M か P)。shards_SET.json の K 番目の (問題, 設定, 種, 秒) を順に 4 スレッドで解き、
res_SET_sK.jsonl に結果を、logs/SET_問題_設定_s種.log.gz に絞ったログを残す。途中から再開できる。ソルバーは同時に 1 つ。
既定に cfg.py の設定と診断 (`ENOMOTO_MIP_LOG=1`・`ENOMOTO_PROF_TREE_PAR=1`) を足す。"""
import gzip, json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
sys.path.insert(0, str(D))
from cfg import CFG  # noqa: E402
SET, k = sys.argv[1], int(sys.argv[2])
mine = json.loads((D / f"shards_{SET}.json").read_text())["shards"][k]
OUT = D / f"res_{SET}_s{k}.jsonl"; LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
KEEP = re.compile(r"^TRSTAT|^TREEPAR|node diag|new incumbent|restart [0-9]+ \(|TREE restart|tree restart|tree search with|presolve finished|root LP [0-9-]|cut loop done")
th = 4
for p, c, seed, tl in mine:
    if (p, c, seed) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS=str(th), ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", ENOMOTO_PROF_TREE_PAR="1")
    env.update(CFG[c])
    t0 = time.time()
    with tempfile.TemporaryFile("w+") as err:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=tl + 600)
        err.seek(0)
        lines = err.read().splitlines()
    prog = [l for l in lines if re.match(r"^MIP: +[0-9.]+s nodes", l)]
    keep = [l for l in lines if KEEP.search(l)] + prog[-1:]
    (LOGS / f"{SET}_{p}_{c}_s{seed}.log.gz").write_bytes(gzip.compress(("\n".join(keep) + "\n").encode()))
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "threads": th, "seed": seed, "tl": tl, "result": res, "wall": round(time.time() - t0, 1), "shard": k, "nproc": os.cpu_count()}) + "\n")
    print(p, c, seed, res.get("status"), res.get("time"), flush=True)
