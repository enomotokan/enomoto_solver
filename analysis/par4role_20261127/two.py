"""6.5 節: supportcase7 と CMS750_4 を、既定と ENOMOTO_MIP_PAR4ROLE=<値> で解く (4 スレッド・600 秒・種 1〜3)。
python two.py [<値>=d] → two_<値>.jsonl (途中から再開できる)。ENOMOTO_PROF_TREE_PAR=1 を付け、ログは logs/two_<問題>_<cfg>_s<種>.log.gz"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
V = sys.argv[1] if len(sys.argv) > 1 else "d"
OUT = D / f"two_{V}.jsonl"
rows = [json.loads(l) for l in OUT.read_text().splitlines()] if OUT.exists() else []
done = {(r["problem"], r["seed"], r["cfg"]) for r in rows}
commit = subprocess.run(["git", "rev-parse", "--short=9", "HEAD"], cwd=WT, capture_output=True, text=True).stdout.strip()
for seed in (1, 2, 3):
    for p in ("CMS750_4", "supportcase7"):
        for cfg in (V, "def"):
            if (p, seed, cfg) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1", "ENOMOTO_PROF_TREE_PAR": "1"})
            if cfg != "def": env["ENOMOTO_MIP_PAR4ROLE"] = cfg
            t0 = time.time()
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True)
            res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            (D / "logs").mkdir(exist_ok=True)
            with gzip.open(D / "logs" / f"two_{p}_{cfg}_s{seed}.log.gz", "wt") as f: f.write(pr.stderr)
            ts = [float(x) for x in re.findall(r"TRSTAT: tree start outer ([\d.]+)", pr.stderr)]
            ar = re.findall(r"after root heuristics: incumbent Some\(([-\d.e+]+)\)", pr.stderr)
            r = {"problem": p, "seed": seed, "cfg": cfg, "result": res, "commit": commit, "tree_start": ts[-1] if ts else None,
                 "inc_after_root": float(ar[-1]) if ar else None, "s_lines": sum("PAR4ROLE s:" in l for l in pr.stderr.splitlines()), "wall": round(time.time() - t0, 1)}
            rows.append(r)
            with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
            print(json.dumps({k: r[k] for k in ("problem", "seed", "cfg", "tree_start", "inc_after_root", "s_lines")} | {"status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes")}), flush=True)
print("TWO_DONE", flush=True)
