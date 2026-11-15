"""MIP の畳める数を数える (ENOMOTO_MIP_FOLD=<値>c、解かない)。python mip_count.py <出力.jsonl> <値> <mps ...>"""
import json, os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
out, val = sys.argv[1], sys.argv[2]
done = set()
if os.path.exists(out):
    done = {json.loads(l)["name"] for l in open(out)}
for mps in sys.argv[3:]:
    name = Path(mps).name.split(".")[0]
    if name in done:
        continue
    env = dict(os.environ, ENOMOTO_MIP_FOLD=val + "c")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", mps, "--time-limit", "10", "--threads", "1"],
                           env=env, capture_output=True, text=True, timeout=600)
        lines = [l for l in r.stderr.splitlines() if l.startswith("MIP_FOLD")]
    except subprocess.TimeoutExpired:
        lines = ["timeout"]
    with open(out, "a") as f:
        f.write(json.dumps({"name": name, "val": val, "fold": lines, "wall": round(time.time() - t0, 1)}) + "\n")
    print(name, lines[-1:] if lines else None, flush=True)
