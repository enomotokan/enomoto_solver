"""切り分けの 1 回ずつの実行: python ab.py 問題 秒 スレッド 種 設定 [設定 ...] (同時に --par 本まで並べる、既定 1)。
結果は ab.jsonl に足す (同じ (問題, 秒, スレッド, 種, 設定) は飛ばす)。v019 は <repo>/.v019env/bin/python で動かす。"""
import json, os, subprocess, sys, time, concurrent.futures as cf
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
args = [a for a in sys.argv[1:] if not a.startswith("--par=")]
par = int(next((a.split("=")[1] for a in sys.argv[1:] if a.startswith("--par=")), "1"))
p, tl, th, seed = args[0], int(args[1]), args[2], int(args[3])
cfgs = args[4:]
OUT = D / "ab.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["tl"], r["threads"], r["seed"], r["cfg"]))
def run(c):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[c]); env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    py = str(WT / ".v019env/bin/python") if c == "v019" else str(WT / ".venv/bin/python")
    t0 = time.time()
    r = subprocess.run([py, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(tl), "--threads", th],
                       cwd=WT, env=env, capture_output=True, text=True)
    js = [l for l in r.stdout.splitlines() if l.startswith("{")]
    res = json.loads(js[-1]) if js else {"status": "crash", "rc": r.returncode}
    rec = {"problem": p, "tl": tl, "threads": th, "seed": seed, "cfg": c, "par": par, "wall": round(time.time() - t0, 1),
           "result": {k: res.get(k) for k in ("status", "objective", "bound", "nodes", "time", "root_bound")}}
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(p, c, res.get("status"), round(res.get("time") or -1, 2), res.get("nodes"), flush=True)
todo = [c for c in cfgs if (p, tl, th, seed, c) not in done]
with cf.ThreadPoolExecutor(par) as ex: list(ex.map(run, todo))
