"""MIPLIB の各問題を解き、双対単体法の求解ごとの「確定ピボットを挟まない破棄の連続の最大」を集計する。
使い方: mipdisc.py 出力.jsonl 制限秒 [ENV=V ...]"""
import sys, os, json, subprocess, re, time
from concurrent.futures import ThreadPoolExecutor
REPO = "/home/user/enomoto_solver"
sys.path.insert(0, REPO + "/scripts")
import run_miplib_benchmark as r
out, tl = sys.argv[1], sys.argv[2]
extra = dict(e.split("=", 1) for e in sys.argv[3:])
W = '''
import sys, json, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
res = m.solve(None, False, float(sys.argv[2]), None, None, 1)
print("RES " + json.dumps({"status": res.get("status"), "obj": res.get("objective"), "nodes": res.get("nodes"), "time": time.perf_counter() - t}), flush=True)
'''
def run(name):
    env = dict(os.environ, ENOMOTO_DEBUG_DISCARD_MAX="1", **extra)
    p = subprocess.run([REPO + "/.venv/bin/python", "-c", W, f"{REPO}/.miplib_cache/{name}.mps", tl], capture_output=True, text=True, env=env, timeout=float(tl) * 3 + 60)
    ms = [int(x) for x in re.findall(r"DISCARD_MAX (\d+)", p.stderr)]
    rec = {"p": name, "solves": len(ms), "max": max(ms, default=0), "ge2": sum(x >= 2 for x in ms), "ge10": sum(x >= 10 for x in ms),
           "exhausted": p.stderr.count("exhausted"), "bail": p.stderr.count("discarded pivots in a row")}
    m = re.search(r"RES (\{.*\})", p.stdout)
    if m:
        rec.update(json.loads(m.group(1)))
    return rec
with ThreadPoolExecutor(4) as ex:
    for rec in ex.map(run, r.PROBLEMS):
        print(json.dumps(rec), flush=True)
        open(out, "a").write(json.dumps(rec) + "\n")
