"""ビルドの対象 CPU の比べ: base (指定なし = x86-64) と v3 (RUSTFLAGS="-C target-cpu=x86-64-v3") の wheel を別の場所 (pip install --target) に入れ、
PYTHONPATH で切り替えて 1 スレッド・種 1 で交互に 2 回ずつ解く。python cpu.py SITE_BASE SITE_V3 → cpu.jsonl"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
SITES = {"base": sys.argv[1], "v3": sys.argv[2]}
PROBS = [("mas76", 60), ("h80x6320d", 60), ("neos-911970", 60), ("pk1", 60), ("timtab1", 60), ("supportcase26", 60), ("noswot", 60)]
OUT = D / "cpu.jsonl"
for rep in (1, 2):
    for p, tl in PROBS:
        for b in (("base", "v3") if rep == 1 else ("v3", "base")):
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(PYTHONPATH=SITES[b], ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1")
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(tl), "--threads", "1"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash"}  # noqa: BLE001
            with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "build": b, "rep": rep, "result": res}) + "\n")
            print(p, b, rep, res.get("status"), res.get("nodes"), res.get("time"), flush=True)
