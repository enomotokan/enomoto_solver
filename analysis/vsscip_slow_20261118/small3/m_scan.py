"""M の 48 問のうち S3GAP が働く問題を探す: ENOMOTO_MIP_S3GAP=0.002・4 スレッド・種 1 で根だけ (ノードの上限 1) を解き、
外側の根のヒューリスティクスが終わった ("after root heuristics" の最初の行) か `--cap` 秒 (既定 180) で止める。
S3GAP の行 (省いたヒューリスティクス) を数えて m_scan.jsonl に 1 問 1 行。ログは logs/mscan_<問題>.log (gzip しない、コミットしない)。
python m_scan.py [--cap 秒] [問題 ...]"""
import json, os, subprocess, sys, time, argparse
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OCODE = """
import sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
mo, _, _ = bh._build_our_model(lp)
mo.solve(None, False, 600.0, None, 1, 4, True)
"""
ap = argparse.ArgumentParser(); ap.add_argument("--cap", type=float, default=180); ap.add_argument("probs", nargs="*")
a = ap.parse_args()
ps = a.probs or sorted({t[0] for s in json.loads((D / "shards_MS3G.json").read_text())["shards"] for t in s})
done = set()
if (D / "m_scan.jsonl").exists():
    done = {json.loads(l)["problem"] for l in open(D / "m_scan.jsonl")}
for p in ps:
    if p in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_S3GAP": "0.002"})
    t0 = time.time(); skipped = []; end = None
    pr = subprocess.Popen([sys.executable, "-c", OCODE, str(mps)], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    with open(D / "logs" / f"mscan_{p}.log", "w") as f:
        for l in pr.stdout:
            f.write(f"[{time.time() - t0:7.2f}] {l}")
            if "S3GAP" in l:
                skipped.append(l.split("root heuristic ")[-1].split(":")[0])
            if "after root heuristics" in l and end is None:
                end = round(time.time() - t0, 1)
                break
            if time.time() - t0 > a.cap:
                break
    pr.kill(); pr.wait()
    row = {"problem": p, "skipped": skipped, "root_heur_end": end, "wall": round(time.time() - t0, 1)}
    with open(D / "m_scan.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
