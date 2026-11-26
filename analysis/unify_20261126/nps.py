"""1 本の行の伝播のまとめ (unify 8) の速さの確かめ: 伝播の重い P の問題を 1 スレッド・種 1・60 秒で解き、ノード数・時間・nodes/s を出す。
使い方: python nps.py 名前 [回数 (既定 2)] → nps_<名前>.jsonl に足す。道筋が同じならノード数は同じで、時間 (nodes/s) だけを比べる。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["mas76", "mas74", "gen-ip054", "pk1", "neos5", "noswot", "misc07", "mik-250-20-75-4", "rout", "timtab1", "qiu"]
name = sys.argv[1]; reps = int(sys.argv[2]) if len(sys.argv) > 2 else 2
OUT = D / f"nps_{name}.jsonl"
for rep in range(reps):
    for p in PROBS:
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
        t0 = time.time()
        out = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "1"],
                             cwd=WT, env=env, capture_output=True, text=True).stdout
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        r = {"name": name, "rep": rep, "problem": p, "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes"), "obj": res.get("objective"), "wall": round(time.time() - t0, 1)}
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        print(json.dumps(r), flush=True)
