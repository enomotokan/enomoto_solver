"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、def (今の既定) と old (直す前: ENOMOTO_PRESOLVE_SLACK_COL_RANGE=0・
ENOMOTO_MIP_RESOLVE_SYM_RESET=0) を種 (ENOMOTO_T_MIP_SEED) ごとに 1 回ずつ解き、res58.jsonl に足す (2026-11-12)。問題ごとに設定の順を入れ替える。
ENOMOTO_ の環境変数は消してから与える。ENOMOTO_DEBUG_SLACK_COL=1 で、区間の空な行の組で列を残した回数・前処理なしの解き直しの回数も数える。
途中から再開できる。使い方: python run58.py 種 [種 ...]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "old": {"ENOMOTO_PRESOLVE_SLACK_COL_RANGE": "0", "ENOMOTO_MIP_RESOLVE_SYM_RESET": "0"}}
OUT = D / "res58.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
k = 0
for seed in [int(s) for s in sys.argv[1:]]:
    for p in PROBS:
        k += 1
        for c in (["def", "old"] if k % 2 else ["old", "def"]):
            if (p, c, seed) in done:
                continue
            env = {kk: v for kk, v in os.environ.items() if not kk.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_DEBUG_SLACK_COL="1", ENOMOTO_MIP_LOG="1", **CFGS[c])
            t0 = time.time()
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                e = r.stderr
                res["warn"] = e.count("MIP: WARNING")
                res["slack_kept"] = e.count("DEBUG_SLACK_COL: kept column")
                res["slack_rounds"] = e.count("DEBUG_SLACK_COL: round=")
                res["resolve"] = e.count("solving without presolve")
                res["orig_only"] = e.count("returning the solution feasible only without the symmetry rows")
            except Exception as ex:  # noqa: BLE001
                res = {"status": "crash", "error": str(ex)[:300]}
            with open(OUT, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "wall": round(time.time() - t0, 1), "result": res}) + "\n")
            print(p, c, seed, res.get("status"), res.get("time"), res.get("slack_kept"), res.get("resolve"), flush=True)
