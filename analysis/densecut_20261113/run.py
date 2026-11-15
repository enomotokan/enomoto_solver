"""densecut の計測: python run.py OUT.jsonl TL CFG[,CFG..] SEEDS(1,2) [問題,...|P|M]。
CFG は cfg.py の CFGS の名前。どの回にも診断 ENOMOTO_MIP_DENSECUT_DIAG=1 を付け、stderr の DENSECUT 行を結果に入れる。
途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)。問題の順に (問題, 種) ごとに設定を並べて回す (時期のずれを対で消す)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS, MSET  # noqa: E402
from probs import PROBS  # noqa: E402
out, tl, cfgs, seeds = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3].split(","), [int(s) for s in sys.argv[4].split(",")]
sel = sys.argv[5] if len(sys.argv) > 5 else "P"
probs = PROBS if sel == "P" else MSET if sel == "M" else sel.split(",")
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
def run(cfg, seed, mps):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_DENSECUT_DIAG"] = "1"
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
        so, se = pr.stdout, pr.stderr
    except subprocess.TimeoutExpired:
        so, se = "", ""
    try: res = json.loads([l for l in so.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    dc = [l for l in se.splitlines() if l.startswith("DENSECUT")]
    warn = [l for l in se.splitlines() if "WARNING" in l][:5]
    return res, dc, warn, round(time.time() - t0, 1)
for p in probs:
    for seed in seeds:
        for cfg in cfgs:
            if (p, cfg, seed) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            res, dc, warn, wall = run(cfg, seed, mps)
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "dc": dc, "warn": warn, "wall": wall, "at": time.strftime("%H:%M:%S")}) + "\n")
            print(p, cfg, seed, res.get("status"), res.get("time"), flush=True)
