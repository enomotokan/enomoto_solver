"""θ の掃引の分担を回す: python run.py K。shards.json の K 番目の (問題, 設定, 種, 秒) を順に 4 スレッドで解き、res_sK.jsonl に結果と
CLQ_LAZY の行 (外した数・戻した数・解き直しの回数)・node diag の行を残す。途中から再開できる。ソルバーは同時に 1 つ。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
k = int(sys.argv[1])
mine = json.loads((D / "shards.json").read_text())["shards"][k]
OUT = D / f"res_s{k}.jsonl"
done = {(r["problem"], r["cfg"], r["seed"]) for r in (json.loads(l) for l in OUT.read_text().splitlines() if l.strip())} if OUT.exists() else set()
KEEP = re.compile(r"CLQ_LAZY|node diag|TRSTAT: tree start")
for p, c, seed, tl in mine:
    if (p, c, seed) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1"); env.update(CFGS[c])
    t0 = time.time()
    with tempfile.TemporaryFile("w+") as err:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=tl + 600)
        err.seek(0); lines = [l for l in err.read().splitlines() if KEEP.search(l)]
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "tl": tl, "result": res, "lines": lines, "wall": round(time.time() - t0, 1), "shard": k}) + "\n")
    print(p, c, seed, res.get("status"), res.get("time"), flush=True)
