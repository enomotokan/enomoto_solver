"""試作 ENOMOTO_MIP_CLQ_LAZY の手元の確かめ (treeclose 9.79 節): python local.py > local.out。6 問 × {def, lazy} × 4 スレッド・種 1、
res_local.jsonl に結果と CLQ_LAZY・node diag の行を残す。途中から再開できる。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = [("neos-827175", 120), ("physiciansched6-2", 120), ("neos-933966", 300), ("mzzv42z", 300), ("mzzv11", 300), ("neos-3216931-puriri", 300)]
CFG = {"def": {}, "lazy": {"ENOMOTO_MIP_CLQ_LAZY": "1"}}
OUT = D / "res_local.jsonl"
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
KEEP = re.compile(r"CLQ_LAZY|node diag|TRSTAT: tree start")
for p, tl in PROBS:
    for c, e in CFG.items():
        if (p, c) in done: continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1"); env.update(e)
        t0 = time.time()
        with tempfile.TemporaryFile("w+") as err:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=tl + 600)
            err.seek(0); lines = [l for l in err.read().splitlines() if KEEP.search(l)]
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "tl": tl, "result": res, "lines": lines, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), res.get("nodes"), flush=True)
