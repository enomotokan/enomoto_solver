"""P2-INF: 特徴の数え上げ (`ENOMOTO_MIP_FEAT_DIAG`)。CFX・CFX2 で測った P・M の問題を 1 スレッド・種 1 で回し、主の木のノード 500 での
FEAT の行 (なければ終わりの行) を feat.jsonl に足す (途中から再開できる)。python featscan.py [秒=60] [並べる数=4]"""
import json, glob, os, subprocess, sys, re
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
tl = sys.argv[1] if len(sys.argv) > 1 else "60"; par = int(sys.argv[2]) if len(sys.argv) > 2 else 4
probs = sorted({json.loads(l)["problem"] for f in glob.glob(str(D / "res_PCFX2_s*.jsonl")) + glob.glob(str(D / "res_MCFX2_s*.jsonl")) for l in open(f)})
out = D / "feat.jsonl"
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
def run(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_FEAT_DIAG": "500", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"})
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", "1"],
                       cwd=WT, env=env, capture_output=True, text=True)
    lines = [l for l in r.stderr.splitlines() if l.startswith("FEAT ") and "tid=0" in l]
    snap = next((l for l in lines if l.startswith("FEAT snapshot")), None) or (lines[-1] if lines else None)
    row = {"problem": p}
    if snap:
        for k, v in re.findall(r"(\w+)=(\S+)", snap):
            row[k] = v
        row["kind"] = snap.split()[1]
    return row
with ThreadPoolExecutor(par) as ex:
    for row in ex.map(run, [p for p in probs if p not in done]):
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row.get("problem"), row.get("kind"), row.get("nodes"), flush=True)
