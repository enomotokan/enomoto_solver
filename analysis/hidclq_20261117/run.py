"""隠れたクリーク型の行の数え上げ (treeclose 9.81 節): python run.py > run.out。58 問と集合詰め込み・分割の多い重い問題を、1 スレッドで根の LP まで解き
(`ENOMOTO_MIP_DIAG_HIDCLQ=stop`、数えたら止める)、res.jsonl に HIDCLQ の行と根の LP・クリークの拡張の行を残す。途中から再開できる。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
HEAVY = ["mzzv11", "neos-3216931-puriri", "neos-933966", "supportcase33", "irp", "satellites2-60-fs", "sct2", "csched008", "neos-3627168-kasai", "tbfp-network"]
# python run.py drop: 余裕のある行を外した LP の比べを、1 回目で余裕のある行があった問題だけ回し直す (res_drop.jsonl)
DROP = len(sys.argv) > 1 and sys.argv[1] == "drop"
OUT = D / ("res_drop.jsonl" if DROP else "res.jsonl")
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
KEEP = re.compile(r"HIDCLQ|MIP: root LP [-0-9]|root LP size|clique extension:|presolve finished")
SLACK = set()
if DROP:
    for l in open(D / "res.jsonl"):
        r = json.loads(l)
        if any(re.search(r"slack at the root LP [1-9]", x) for x in r["lines"]): SLACK.add(r["problem"])
for p in dict.fromkeys(PROBS + HEAVY):
    if p in done or DROP and p not in SLACK: continue
    tl = 900 if p in HEAVY else 300
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DIAG_HIDCLQ="stop")
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
    print(p, round(time.time() - t0, 1), sum("HIDCLQ" in l for l in lines), flush=True)
