"""wide2 M の fix の遅れ (neos-933966・neos-1171448) と rail01 の下界の切り分け (親の依頼、2026-11-21): 各 600 秒 × def・fix × 種 1 × 2 回、4 スレッド。
ログ (ENOMOTO_MIP_LOG=1 の stderr) は t78/logs/<問題>_<設定>_s<種>_r<回>.log、結果は t78/res.jsonl (続きから回せる)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[3]
sys.path.insert(0, str(D.parent))
from cfg import CFGS  # noqa: E402
OUT = D / "res.jsonl"; (D / "logs").mkdir(exist_ok=True)
done = {(r["problem"], r["cfg"], r["seed"], r["rep"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for prob, tl in (("neos-933966", 600), ("neos-1171448", 600), ("rail01", 600)):
  mps = WT / ".miplib_cache" / f"{prob}.mps"
  for rep in (1, 2):
    for seed in (1,):
        for cfg in ("def", "fix"):
            if (prob, cfg, seed, rep) in done: continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CFGS[cfg]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
            log = D / "logs" / f"{prob}_{cfg}_s{seed}_r{rep}.log"
            with open(log, "w") as fe:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                                    cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash"}  # noqa: BLE001
            with open(OUT, "a") as f:
                f.write(json.dumps({"problem": prob, "cfg": cfg, "seed": seed, "rep": rep, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
            print(prob, cfg, seed, rep, res.get("status"), res.get("time"), flush=True)
