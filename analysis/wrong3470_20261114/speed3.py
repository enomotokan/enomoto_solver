"""直しが働く 3 問を直す前 (old: 直しを環境変数で切る) と後 (def) で種 1〜3・TL 秒・4 スレッドで解く (python3 speed3.py [TL=60])。
speed3.jsonl に足す。途中から再開できる。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 60.0
CF = {"def": {}, "old": {"ENOMOTO_T_EMPTY_ROW_FEAS_TOL": "0", "ENOMOTO_PRESOLVE_NO_INT_DOUBLETON_SNAP": "1"}}
OUT = D / "speed3.jsonl"
done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
for seed in (1, 2, 3):
    for p in ("p200x1188c", "drayage-100-23", "drayage-25-23"):
        for cfg in ("def", "old"):
            if (p, cfg, seed) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CF[cfg]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed))
            t0 = time.time()
            pr = subprocess.run([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, errors="replace", timeout=TL + 600)
            res = next((json.loads(l) for l in reversed(pr.stdout.splitlines()) if l.startswith("{")), {"status": "crash"})
            rec = {"problem": p, "cfg": cfg, "seed": seed, "wall": round(time.time() - t0, 1), **{k: res.get(k) for k in ("status", "objective", "bound", "time", "nodes")}}
            print(rec, flush=True)
            with OUT.open("a") as f: f.write(json.dumps(rec) + "\n")
