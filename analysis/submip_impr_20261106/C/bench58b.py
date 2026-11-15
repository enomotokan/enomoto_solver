"""担当 C の改良 (11-08) の害の確かめ (bench58.py の写し、結果は bench58b.jsonl): 58 問を 4 スレッド・60 秒で設定ごとに解く。
analysis/halve_submip_20261030/bench.py と同じ形 (設定の順は回ごとに逆、途中から再開できる)。種は ENOMOTO_T_MIP_SEED=回+1。
使い方: python bench58.py <設定名,...> <回>"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
C = {l.split(":")[0]: l.split(":", 1)[1].strip() for l in (D / "cfgs.txt").read_text().splitlines() if l and not l.startswith("#")}
CFGS = {k: dict(a.split("=", 1) for a in v.split()) for k, v in C.items()}
OUT = D / "bench58b.jsonl"
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
if rep % 2:
    cfgs = cfgs[::-1]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for p in PROBS:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(rep + 1), **CFGS[c])
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)
