"""試作 ENOMOTO_MIP_PROOFCOST (p・a・pa) と既定を、1 スレッド・種 1・60 秒で比べる。ENOMOTO_PROF_TREE_PAR=1 の TREEPAR の行から証明の数
(生きている本数・評価・刈り込み・締め付け・差分の更新) を読む。python run.py [種] → res.jsonl"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
PROBS = ["mas76", "pk1", "noswot", "timtab1", "supportcase26", "neos5", "mas74"]
CF = {"def": {}, "p": {"ENOMOTO_MIP_PROOFCOST": "p"}, "a": {"ENOMOTO_MIP_PROOFCOST": "a"}, "pa": {"ENOMOTO_MIP_PROOFCOST": "pa"}}
seed = int(sys.argv[1]) if len(sys.argv) > 1 else 1
# 2 つ目の引数で設定を絞る (例 def,pa)
if len(sys.argv) > 2: CF = {k: v for k, v in CF.items() if k in sys.argv[2].split(",")}
OUT = D / "res.jsonl"
done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for p in PROBS:
    for c, ex in CF.items():
        if (p, c, seed) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ex); env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_PROF_TREE_PAR="1")
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "60", "--threads", "1"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        m = re.search(r"proofs own/imp: live (\d+)/\d+ eval (\d+)/\d+ prune (\d+)/\d+ tight (\d+)/\d+ absorb (\d+)/\d+", pr.stderr)
        pf = dict(zip(("live", "eval", "prune", "tight", "absorb"), map(int, m.groups()))) if m else {}
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "result": res, "proofs": pf}) + "\n")
        print(p, c, seed, res.get("status"), res.get("nodes"), round(res.get("time") or 0, 1), pf, flush=True)
