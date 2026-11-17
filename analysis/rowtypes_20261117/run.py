"""元の問題の行の種類ごとの数え上げ (treeclose 9.83 節): python run.py > run.out。58 問 (90 秒) と重い問題 (150 秒) を 4 スレッド・種 1 で解き
(`ENOMOTO_MIP_DIAG_ROWTYPES=1`、木の途中の数え上げは 60 秒)、res.jsonl に ROWTYPES の行を残す。途中から再開できる。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
HEAVY = ["mzzv11", "neos-3216931-puriri", "neos-933966", "supportcase33", "irp", "satellites2-60-fs", "sct2", "csched008", "neos-3627168-kasai", "tbfp-network"]
OUT = D / "res.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
for p in dict.fromkeys(PROBS + HEAVY):
    if p in done: continue
    tl = 150 if p in HEAVY else 90
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DIAG_ROWTYPES="1")
    t0 = time.time()
    with tempfile.TemporaryFile("w+") as err:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=tl + 600)
        err.seek(0); lines = [l for l in err.read().splitlines() if "ROWTYPES" in l]
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "heavy": p in HEAVY, "tl": tl, "result": res, "lines": lines, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, res.get("status"), round(time.time() - t0, 1), len(lines), flush=True)
