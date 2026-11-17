"""シグモイドの a・b を合わせるための根の計測 (README の 2 節): python sigfit_run.py [問題,...]
58 問 + mzzv11・irp・tbfp-network を、c18 (既定の fallback) と c18 + ENOMOTO_MIP_CLIQUE_CUTS=root で根まで (node_limit=1・4 スレッド・300 秒) 解き、
根の下界・時間と、ENOMOTO_MIP_DEBUG_CLQSIG の点数 s と 4 つの量を sigfit.jsonl に足す (途中から再開できる)。ソルバーは同時に 1 つ。
ログは CLQSIG・clique の判定・カットのループの終わりの行だけを logs_sig/ に残す。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parents[1] / "cbcimpl_20261029"))
from cfg import C18  # noqa: E402
from probs import PROBS  # noqa: E402
probs = sys.argv[1].split(",") if len(sys.argv) > 1 else PROBS + ["mzzv11", "irp", "tbfp-network"]
OUT = D / "sigfit.jsonl"; LOGS = D / "logs_sig"; LOGS.mkdir(exist_ok=True)
done = {(r["problem"], r["cfg"]) for r in (json.loads(l) for l in (OUT.read_text().splitlines() if OUT.exists() else []) if l.strip())}
OCODE = """
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
mo, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = mo.solve(None, False, 300.0, None, 1, 4, True); t = time.perf_counter() - t
print("RES " + json.dumps({k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "lp_iterations", "root_bound")} | {"time": t}))
"""
KEEP = re.compile(r"CLQSIG|clique fallback|clique sigmoid|cut loop done|clique graph")
CF = {"fb": {}, "root": {"ENOMOTO_MIP_CLIQUE_CUTS": "root"}}
for p in probs:
    mps = WT / ".miplib_cache" / f"{p}.mps"
    if not mps.exists():
        sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb; mps = rb.fetch(p, WT / ".miplib_cache")  # noqa: E401,E702
    for c, extra in CF.items():
        if (p, c) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(C18); env.update(extra); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_MIP_LOG"] = "1"; env["ENOMOTO_MIP_DEBUG_CLQSIG"] = "1"
        t0 = time.time()
        pr = subprocess.run([sys.executable, "-c", OCODE, str(mps)], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
        res = [l for l in pr.stdout.splitlines() if l.startswith("RES ")]
        r = json.loads(res[-1][4:]) if res else {"status": "crash", "returncode": pr.returncode}
        kept = [l for l in pr.stderr.splitlines() if KEEP.search(l)]
        (LOGS / f"{c}_{p}.log").write_text("\n".join(kept) + "\n")
        m = next((re.search(r"CLQSIG s (\S+) sp (\S+) ed (\S+) fr (\S+) vi (\S+)", l) for l in kept if l.startswith("CLQSIG s")), None)
        feats = dict(zip(("s", "sp", "ed", "fr", "vi"), map(float, m.groups()))) if m else {}
        row = {"problem": p, "cfg": c, "wall": round(time.time() - t0, 1)} | r | feats
        with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)
