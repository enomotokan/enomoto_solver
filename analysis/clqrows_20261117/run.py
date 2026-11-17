"""クリーク型の行の数え上げ (treeclose 9.78 節): python run.py > run.out。58 問と集合詰め込み・分割の多い重い問題を、1 スレッドで根の LP まで解き
(`ENOMOTO_MIP_DIAG_CLQROWS=stop`、数えたら止める)、res.jsonl に CLQROWS の 2 行と根の LP・クリークの拡張の行を残す。途中から再開できる。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
HEAVY = ["satellites2-60-fs", "neos-933966", "mzzv11", "mzzv42z", "irp", "supportcase33", "neos-3216931-puriri"]
OUT = D / "res.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
KEEP = re.compile(r"CLQROWS|MIP: root LP [-0-9]|root LP size|clique extension:|presolve finished")
for p in PROBS + HEAVY:
    if p in done: continue
    tl = 900 if p in HEAVY else 300
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DIAG_CLQROWS="stop")
    t0 = time.time()
    with tempfile.TemporaryFile("w+") as err:
        try:
            subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "1"],
                           cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=err, text=True, timeout=tl + 300)
        except subprocess.TimeoutExpired:
            pass
        err.seek(0); lines = [l for l in err.read().splitlines() if KEEP.search(l)]
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "heavy": p in HEAVY, "lines": lines, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, round(time.time() - t0, 1), sum("CLQROWS" in l for l in lines), flush=True)
