"""FPSYM 段階 1: 既定のまま (4 スレッド・種 1) で Feasibility Pump の回数を数える (`ENOMOTO_MIP_FP_STATS=1` の `MIP: FP_STATS` の行)。
python count.py SET 秒 → res_count_SET.jsonl に足す (途中から再開できる)。SET: P (MIPLIB の 58 問、対称性の有無は hang_20261117/symscan_P.txt)、
M (analysis/sstx_20261119 の M の対称な 19 問)。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
SET, TL = sys.argv[1], float(sys.argv[2])
if SET == "P":
    from probs import PROBS
    probs = list(PROBS)
else:
    probs = json.loads((WT / "analysis/sstx_20261119/shards_M.json").read_text())["problems"]
def gens(f):
    g = {}
    for l in open(f):
        m = re.match(r"^([^:]+): MIP: symmetry: (\d+) generators", l)
        if m: g[m.group(1)] = int(m.group(2))
    return g
G = gens(WT / "analysis/hang_20261117/symscan_P.txt"); G.update(gens(WT / "analysis/hang_20261117/symscan_M.txt"))
OUT = D / f"res_count_{SET}.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in probs:
    if p in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_FP_STATS": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1"})
    err = tempfile.TemporaryFile("w+")
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=TL + 300)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    err.seek(0); fl = [l.strip() for l in err.read().splitlines() if l.startswith("MIP: FP_STATS")]
    fp = None
    if fl:
        nums = [int(x) for x in re.findall(r"\b(\d+)\b", fl[-1])]
        keys = ["calls", "found", "passes", "conflicts", "caps", "fixed_not_found", "integral_lp", "rounding", "cycle", "budget", "lp_stop", "passes100", "conflict_pos"]
        fp = dict(zip(keys, nums))
    row = {"problem": p, "gens": G.get(p, 0), "tl": TL, "status": res.get("status"), "time": res.get("time"), "first_solution_time": res.get("first_solution_time"), "fp": fp, "wall": round(time.time() - t0, 1)}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, row["gens"], row["status"], fp, flush=True)
