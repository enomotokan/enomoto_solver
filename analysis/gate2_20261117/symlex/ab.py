"""候補 7 の手元の当たり: 既定 (def、orbital fixing + LexRed) と lex-leader の行 (lex = `ENOMOTO_MIP_SYM_LEXGATE=lex`) を、
scan.jsonl で orbital fixing の生成元がある問題で交互に回す。python ab.py 出力 秒 種,種,... [問題,...]。4 スレッド・1 本ずつ"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
out, tl, seeds = D / sys.argv[1], int(sys.argv[2]), [int(s) for s in sys.argv[3].split(",")]
probs = sys.argv[4].split(",") if len(sys.argv) > 4 else [json.loads(l)["problem"] for l in open(D / "scan.jsonl") if json.loads(l)["feats"]]
CFG = {"def": {}, "lex": {"ENOMOTO_MIP_SYM_LEXGATE": "lex"}}
done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, open(out))} if out.exists() else set()
for seed in seeds:
    for p in probs:
        for cfg, ev in CFG.items():
            if (p, cfg, seed) in done:
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ev); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
            t0 = time.time()
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
            try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
            m = re.search(r"root.*?bound[^\d-]*([-\d.e+]+)", r.stderr)
            rec = {"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "wall": round(time.time() - t0, 1),
                   "sym": [l for l in r.stderr.splitlines() if "symmetry" in l][-3:]}
            with open(out, "a") as f:
                f.write(json.dumps(rec) + "\n")
            print(p, cfg, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound", res.get("dual_bound")), flush=True)
