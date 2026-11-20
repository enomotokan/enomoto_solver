"""前処理だけ (ENOMOTO_MIP_DEBUG_PRETIME=stop) を SPARSIFY_LATE つきで回し、疎化の計数の行 (`MIP: sparsify-late stats`) を pre.jsonl に。
python pre.py 問題,... 札 [K=V ...]"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb  # noqa: E402
probs, tag = sys.argv[1].split(","), sys.argv[2]; kv = [a for a in sys.argv[3:] if "=" in a]
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_DEBUG_PRETIME": "stop"})
    for x in kv:
        k, v = x.split("=", 1); env[k] = v
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True)
    err = pr.stderr; wall = time.time() - t0
    st = re.findall(r"sparsify-late stats m (\d+) n (\d+) bincols (\d+) changed (\d+) binonly (\d+) shrunk<=1 (\d+) nnz (\d+) -> (\d+)", err)
    sizes = re.findall(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+), nonzeros (\d+) -> (\d+)", err)
    pt = re.findall(r"PRETIME total=([\d.]+) m=(\d+) n=(\d+) nnz=(\d+)", err)
    row = {"problem": p, "tag": tag, "env": kv, "wall": round(wall, 2), "stats": [list(map(int, s)) for s in st], "sizes": sizes, "pretime": pt[-1] if pt else None}
    with open(D / "pre.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
