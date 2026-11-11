"""LUCLOCK の LP 単体の計測: MIP の問題の整数性を外した LP (連続緩和) を、1 スレッドで解く。
使い方: python lpbench.py OUT.jsonl {heavy|問題,...} 設定,... [回の数 (既定 1)]
設定は bench.py の CFG と同じ名前。stderr の LUCLOCK の行 (反復 1000 以上の双対単体法) を残す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from bench_cfg import CFG, HEAVY  # noqa: E402
WORK = r'''
import sys, time, json
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
lp.integrality_ = []
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, float(sys.argv[2]), None, None, 1)
print(json.dumps({"status": r["status"], "objective": r["objective"], "time": time.perf_counter() - t}))
'''
OUT, P, cfgs = Path(sys.argv[1]), sys.argv[2], sys.argv[3].split(",")
REPS = int(sys.argv[4]) if len(sys.argv) > 4 else 1
if not OUT.is_absolute():
    OUT = D / OUT
PROBS = HEAVY if P == "heavy" else P.split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["run"]))
for rep in range(REPS):
    for p in PROBS:
        for c in cfgs:
            if (p, c, rep) in done:
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_SIMPLEX_LUCLOCK_LOG": "1"})
            env.update(CFG[c])
            try:
                pr = subprocess.run([sys.executable, "-c", WORK, str(mps), "600"], cwd=WT, env=env, capture_output=True, text=True, timeout=900)
                res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
                err = pr.stderr
            except Exception as e:  # noqa: BLE001
                res, err = {"status": "crash", "error": str(e)[-200:]}, ""
            lc = [json.loads(l[8:]) for l in err.splitlines() if l.startswith("LUCLOCK ")]
            r = {"problem": p, "cfg": c, "run": rep, "result": res, "luclock": lc}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, rep, res.get("status"), round(res.get("time", -1), 3), [(x["iters"], x["builds"], round(x["secs"], 2)) for x in lc], flush=True)
