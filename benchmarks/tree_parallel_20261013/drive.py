"""threads ごとに MIPLIB 40 問を 1 問ずつ解き、TREEPAR の行と結果を JSONL に書く。

使い方: drive.py THREADS TIME_LIMIT OUT.jsonl [NAME ...]
"""
import json
import os
import subprocess
import sys

sys.path.insert(0, "/home/user/enomoto_solver/scripts")
import run_miplib_benchmark as rb

threads, tl, out = int(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
names = sys.argv[4:] or rb.PROBLEMS
done = set()
if os.path.exists(out):
    done = {json.loads(l)["name"] for l in open(out)}
for n in names:
    if n in done:
        continue
    env = dict(os.environ, ENOMOTO_PROF_TREE_PAR="1", ENOMOTO_MIP_THREADS=str(threads))
    cmd = [sys.executable, rb.__file__, "--worker-ours", f"/home/user/enomoto_solver/.miplib_cache/{n}.mps",
           "--time-limit", str(tl), "--threads", str(threads)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=tl * 1.5 + 60, env=env, cwd="/tmp")
        res = next((json.loads(l) for l in reversed(p.stdout.splitlines()) if l.startswith("{")),
                   {"status": f"crash {p.returncode}"})
        prof = [l[9:] for l in p.stderr.splitlines() if l.startswith("TREEPAR: ")]
    except subprocess.TimeoutExpired:
        res, prof = {"status": "killed"}, []
    rec = {"name": n, "threads": threads, **res, "prof": prof}
    with open(out, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(n, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
