"""40 問を既定 (A) と試作 (B: ENOMOTO_MIP_LS_OBJ_STEP=1 ENOMOTO_MIP_FIND_POLL_SB=1) で 1 回ずつ (4 スレッド・60 秒)。
使い方: python side40.py <出力 jsonl> [問題 ...]"""
import json, os, subprocess, sys
from pathlib import Path
here = Path(__file__).resolve().parent; repo = here.parents[1]
sys.path.insert(0, str(repo / "scripts"))
import run_miplib_benchmark as rb
names = sys.argv[2:] or list(json.load(open(repo / "benchmarks/miplib_results_4t_60s_20261020_bc89ad1_highsgap0.json")))
B = {"ENOMOTO_MIP_LS_OBJ_STEP": "1", "ENOMOTO_MIP_FIND_POLL_SB": "1"}
out = open(sys.argv[1], "a")
for n in names:
    mps = rb.fetch(n, repo / ".miplib_cache")
    for tag, extra in [("A", {}), ("B", B)]:
        env = dict(os.environ, **extra)
        try:
            p = subprocess.run([sys.executable, str(here / "run_ours.py"), str(mps), "60"], env=env, capture_output=True, text=True, timeout=200)
            line = [l for l in p.stdout.splitlines() if l.startswith("RESULT")]
            r = json.loads(line[-1][7:]) if line else {"status": "crash", "err": p.stderr[-300:]}
        except subprocess.TimeoutExpired:
            r = {"status": "timeout"}
        out.write(json.dumps({"name": n, "cfg": tag, **r}) + "\n"); out.flush()
        print(n, tag, r.get("status"), r.get("objective"), round(r.get("time") or 0, 1), flush=True)
