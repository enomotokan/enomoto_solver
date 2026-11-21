"""直した後 (pool_arena) の def / pe の測り直し (2026-11-21、README 6 節)。JOBS の (問題, 設定, スレッド, 種, 秒) を順に解き、
ログを logs2/ に gz で置き、統計を抜いて res2.jsonl に足す (途中から再開できる)。抜き方は run.py と同じ。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "pe": {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1"}}
SC = "supportcase26"
CM2S = ["30n20b8", "timtab1", "neos-1582420", "gen-ip054", "neos5", "rococoC10-001000"]
JOBS = [(SC, c, 1, s, 1800) for s in (1, 2) for c in ("def", "pe")] + [(SC, c, 4, s, 600) for s in (1, 2, 3) for c in ("def", "pe")] \
    + [(p, c, 4, s, 600) for p in CM2S for s in (1, 2, 3) for c in ("def", "pe")]
OUT = D / "res2.jsonl"
NUM = r"([-\d.e+]+)"
PAT = {
    "ca": r"conflict analysis \(HiGHS style\): calls (\d+), conflicts (\d+), added (\d+), mean length ([\d.]+), max length (\d+), ([\d.]+)s",
    "proofs": r"dual proofs and conflicts (\d+) \(conflicts added \d+ \(from proofs (\d+)\), Farkas (\d+), pruned (\d+) nodes, tightened (\d+) bounds",
    "lit": r"literal conflicts: added (\d+), in pool (\d+) \(aged out (\d+)\), pruned (\d+) nodes, tightened (\d+) bounds",
    "pool": r"cut pool propagation: calls (\d+), pruned (\d+) nodes, tightened (\d+) bounds, ([\d.]+)s",
    "lev": r"conflict levels \(all threads\): examined (\d+), 1-UIP (\d+), stopped with >= 2 left (\d+).*?decision (\d+), external (\d+), local redcost (\d+), strong branching (\d+), orbitope (\d+), dual proof (\d+), literal conflict (\d+)",
    "ana": r"conflict analyses \(all threads\): (\d+), stopped: first level not resolved (\d+), set too large (\d+), not explained by a row (\d+), no conflict (\d+), 5-level rule (\d+), all levels (\d+); dropped as too long (\d+), conflicts (\d+), levels (\d+)",
    "node": r"node diag: nodes (\d+) node LPs (\d+) mean rows ([\d.]+) iters (\d+) \(([\d.]+)/LP\) LP secs ([\d.]+) sep rounds (\d+) added (\d+) node cut secs ([\d.]+)",
    "ticks": r"MIP: TICKS cpu ([\d.]+) total (\d+) lp (\d+) prop (\d+) sep (\d+) pre (\d+) heur (\d+) node (\d+) lpit (\d+) lu (\d+) lum (\d+)",
    "tree": r"TRSTAT: tree start outer ([\d.]+) solve ([\d.]+)",
    "finish": r"MIP: finish start \w+ \(([\d.]+)s\)",
    "branch": r"branching: (\d+) selections, (\d+) without strong branching.*?sb LPs (\d+) iters (\d+)",
}
PROG = re.compile(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb " + NUM + " ub " + NUM)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["threads"], r["seed"]))
(D / "logs2").mkdir(exist_ok=True)
for P, cfg, th, seed, tl in JOBS:
    if (P, cfg, th, seed) in done: continue
    mps = rb.fetch(P, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg], ENOMOTO_MIP_THREADS=str(th), ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CONFLICT_LEV_DIAG="1")
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 2 + 600)
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    log = D / "logs2" / f"{P}_{cfg}_t{th}_s{seed}.log.gz"
    with gzip.open(log, "wt") as f: f.write(r.stderr)
    rec = {"problem": P, "cfg": cfg, "threads": th, "seed": seed, "tl": tl, "result": res, "wall": round(time.time() - t0, 1)}
    for k, pat in PAT.items():
        m = re.findall(pat, r.stderr, re.S if k in ("lev",) else 0)
        rec[k] = [float(x) for x in (m[-1] if isinstance(m[-1], tuple) else (m[-1],))] if m else None
    rec["progress"] = [[float(a), int(b), int(c), float(d), float(e)] for a, b, c, d, e in PROG.findall(r.stderr)]
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(P, cfg, th, seed, res.get("status"), res.get("time"), res.get("nodes"), rec["pool"], flush=True)
