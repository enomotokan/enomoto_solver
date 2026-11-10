"""FPR の試し: python run1.py OUT.jsonl CFG SEED TL prob1 prob2 ...
CFG: def (環境変数なし) / fpr (ENOMOTO_MIP_FPR=1) / それ以外は "K=V,K=V"。1 回ずつ順に解き、結果と FPR の行の要約を OUT に足す (途中から再開できる)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "fpr": {"ENOMOTO_MIP_FPR": "1"}, "fprt": {"ENOMOTO_MIP_FPR": "t"}, "fpr2": {"ENOMOTO_MIP_FPR": "1"}, "fprp": {"ENOMOTO_MIP_FPR": "p"}, "fprr": {"ENOMOTO_MIP_FPR": "rb"},
        "fprbandit": {"ENOMOTO_MIP_FPR": "1", "ENOMOTO_MIP_NOINC_BANDIT": "1"}}
out, cfg, seed, tl = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
probs = sys.argv[5:]
OUT = Path(out)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
env0 = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
for p in probs:
    if (p, cfg, seed) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(env0)
    env.update(CFGS[cfg] if cfg in CFGS else dict(kv.split("=", 1) for kv in cfg.split(",")))
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed)})
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 600)
    try:
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    fl = [l for l in pr.stderr.splitlines() if "FPR" in l and "FPR run" not in l][:12]
    # 誤答の検査用: 暫定解と下界
    inc = [l for l in pr.stderr.splitlines() if "new incumbent" in l or "FJ thread" in l or "feasibility jump" in l][:10]
    rec = {"incumbent_lines": inc, "problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "fpr_lines": fl, "wall": round(time.time() - t0, 1)}
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), res.get("first_solution_time"), res.get("objective"), flush=True)
