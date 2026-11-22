"""衝突の行の使われ方の診断 (2026-11-22、README 1 節)。python diag.py 名前 秒 [環境変数=値 ...] [-- 問題 ...]:
問題ごとに 1 スレッド・種 1・`ENOMOTO_MIP_TIMEDEP=b` (決定的な設定)・`ENOMOTO_MIP_CONFDEEP_DIAG=1`・`ENOMOTO_MIP_CONFLICT_LEV_DIAG=1` で解き、
ログを logs/<名前>_<問題>.log.gz に置き、結果と統計の行を diag_<名前>.jsonl に足す (途中から再開できる)。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
args = sys.argv[1:]
NAME, TL = args[0], float(args[1])
rest = args[2:]
probs = rest[rest.index("--") + 1:] if "--" in rest else []
envs = dict(a.split("=", 1) for a in (rest[:rest.index("--")] if "--" in rest else rest))
PS = probs or ["bnatt400", "supportcase26", "timtab1", "mcsched", "neos5", "noswot", "eil33-2", "neos859080", "10teams", "fhnw-binpack4-4", "fiball", "neos-1456979", "pk1"]
OUT = D / f"diag_{NAME}.jsonl"
KEYS = ("CONFDEEP", "literal conflicts:", "conflict analysis (HiGHS style)", "conflict levels (all threads)", "conflict analyses (all threads)", "node diag", "TICKS cpu", "dual proofs and conflicts", "lit explain budget")
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
(D / "logs").mkdir(exist_ok=True)
for P in PS:
    if P in done: continue
    mps = rb.fetch(P, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_TIMEDEP="b", ENOMOTO_MIP_LOG="1",
               ENOMOTO_MIP_CONFDEEP_DIAG="1", ENOMOTO_MIP_CONFLICT_LEV_DIAG="1", **envs)
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "1"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=TL * 2 + 600)
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    with gzip.open(D / "logs" / f"{NAME}_{P}.log.gz", "wt") as f: f.write(r.stderr)
    lines = {}
    for l in r.stderr.splitlines():
        for k in KEYS:
            if k in l and "sub-MIP" not in l:
                lines[l.split(":", 2)[1].strip()[:40] if k == "CONFDEEP" else k] = l
    rec = {"problem": P, "name": NAME, "tl": TL, "env": envs, "result": res, "wall": round(time.time() - t0, 1), "lines": lines}
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(P, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
