"""kasai の nra の害 (2026-11-26、親の依頼): python analysis/nrakasai_20261126/run.py OUT.jsonl 問題 スレッド数 種 CFG[,CFG...] [--tl 秒]
設定: def・c2cp (c2c の nra を r0k23p)・c2cp-nra (c2cp から nra を抜く)・c2cp+X (c2cp に環境変数 X=V を足す: "c2cp+ENOMOTO_...=v")。
stderr はまるごと logs/ に残す (ENOMOTO_PROF_TREE_PAR・ENOMOTO_MIP_NRA_TRACE=2000 をつける。解き方は変えない)。同じ行があれば飛ばす。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/combo2_20261125"))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import C2  # noqa: E402
C2CP = dict(C2, ENOMOTO_MIP_SB_PROBEBD="1", ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p")


def cfg_env(c):
    if c == "def": return {}
    if c == "c2cp": return dict(C2CP)
    if c == "c2cp-nra": return {k: v for k, v in C2CP.items() if k != "ENOMOTO_MIP_NODEROWS_ADAPT"}
    if c.startswith("c2cp+"): return dict(C2CP, **dict(a.split("=", 1) for a in c[5:].split("+")))
    raise ValueError(c)


args = sys.argv[1:]
TL = 600.0
if "--tl" in args:
    i = args.index("--tl"); TL = float(args[i + 1]); del args[i:i + 2]
OUT = Path(args[0]); prob, th, seed = args[1], args[2], int(args[3]); cfgs = args[4].split(",")
mps = rb.fetch(prob, WT / ".miplib_cache")
done = {(r["problem"], r["cfg"], r["threads"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
for c in cfgs:
    if (prob, c, th, seed) in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(cfg_env(c)); env.update(ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_PROF_TREE_PAR="1", ENOMOTO_MIP_NRA_TRACE="2000")
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", th],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 900)
    (D / "logs" / f"{prob}_{c.replace('=', '_')}_t{th}_s{seed}.err").write_text(pr.stderr)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": pr.returncode}  # noqa: BLE001
    row = {"problem": prob, "cfg": c, "threads": th, "seed": seed, "tl": TL, "result": res, "wall": round(time.time() - t0, 1)}
    with OUT.open("a") as f: f.write(json.dumps(row) + "\n")
    print(prob, th, seed, c, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
