"""4 スレッド・種 1〜3 で base / lev99 / age10k / short を比べる (2026-11-22、README の通しの節)。
JOBS の (問題, 設定, 種) を順に解き、ログを logs4/ に gz で置き、結果を res4.jsonl に足す (途中から再開できる)。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"base": {}, "lev99": {"ENOMOTO_T_MIP_CONFLICT_LEVELS": "1000"}, "age10k": {"ENOMOTO_T_MIP_LIT_CONFLICT_AGE": "10000"},
        "short": {"ENOMOTO_T_MIP_CONFLICT_LEN_INT": "0.1", "ENOMOTO_T_MIP_CONFLICT_LEN_BASE": "10"}}
for a in sys.argv[1:]:  # 名前=ENV=値,ENV=値 で形を足す
    n, kv = a.split("=", 1); CFGS[n] = dict(x.split("=", 1) for x in kv.split(","))
PROBS = ["timtab1", "neos5", "noswot", "eil33-2", "neos859080", "10teams", "fhnw-binpack4-4", "pk1"]
TL = 300
JOBS = [(p, c, s) for s in (1, 2, 3) for p in PROBS for c in CFGS]
OUT = D / "res4.jsonl"
PAT = {"lit": r"literal conflicts: added (\d+), in pool (\d+) \(aged out (\d+)\), pruned (\d+) nodes, tightened (\d+) bounds",
       "ca": r"conflict analysis \(HiGHS style\): calls (\d+), conflicts (\d+), added (\d+), mean length ([\d.]+), max length (\d+), ([\d.]+)s"}
done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
(D / "logs4").mkdir(exist_ok=True)
for P, cfg, seed in JOBS:
    if (P, cfg, seed) in done: continue
    mps = rb.fetch(P, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg], ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=TL * 2 + 600)
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    with gzip.open(D / "logs4" / f"{P}_{cfg}_s{seed}.log.gz", "wt") as f: f.write(r.stderr)
    rec = {"problem": P, "cfg": cfg, "seed": seed, "env": CFGS[cfg], "result": res, "wall": round(time.time() - t0, 1)}
    for k, pat in PAT.items():
        m = re.findall(pat, r.stderr)
        rec[k] = [float(x) for x in m[-1]] if m else None
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(P, cfg, seed, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
