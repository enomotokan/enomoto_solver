"""停滞の判定に距離の総和 F を使う試作 ENOMOTO_MIP_CUT_STALL_FRAC (s・c) と既定を、1 スレッド・種 1・120 秒で比べる (ENOMOTO_MIP_CUTSTOP_LOG=1)。
python run.py → res.jsonl・logs/<問題>_<設定>.log (続きから回せる)"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["swath1", "beasleyC3", "mik-250-20-75-4", "timtab1", "mc11", "roll3000", "neos17", "neos-911970", "neos-1582420", "h80x6320d"]
CF = {"def": {}, "s": {"ENOMOTO_MIP_CUT_STALL_FRAC": "s"}, "c": {"ENOMOTO_MIP_CUT_STALL_FRAC": "c"}}
OUT = D / "res.jsonl"
done = {(r["problem"], r["cfg"], r.get("seed", 1)) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
# 種 1 で根が変わった 5 問は、種 2・3 の def・s も回す (python run.py 2)
SEEDS = (1,) if len(sys.argv) < 2 else (1, 2, 3)
RUNS = [(p, c, 1) for p in PROBS for c in CF] + [(p, c, s) for s in SEEDS if s > 1 for p in ("swath1", "mik-250-20-75-4", "neos-911970", "neos-1582420", "h80x6320d") for c in ("def", "s")]
for p, c, seed in RUNS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    ex = CF[c]
    if (p, c, seed) in done: continue
    if True:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ex); env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CUTSTOP_LOG="1")
        log = D / "logs" / (f"{p}_{c}.log" if seed == 1 else f"{p}_{c}_s{seed}.log")
        with open(log, "w") as fe:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        stops = [json.loads(l[len("CUTSTOP "):]) for l in log.read_text().splitlines() if l.startswith("CUTSTOP ")]
        roots = [{"reason": s.get("reason"), "rounds": s.get("rounds"), "obj1": s.get("obj1"), "restarts": s.get("restarts")} for s in stops]
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "result": res, "roots": roots}) + "\n")
        print(p, c, seed, res.get("status"), res.get("time"), res.get("nodes"), [(r["reason"], r["rounds"]) for r in roots], flush=True)
