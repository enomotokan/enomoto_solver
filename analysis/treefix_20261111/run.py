"""TREEFIX の計測 (analysis/treefix_20261111.md): 問題 x 設定 x 種を 4 スレッドで 1 つずつ解き、jsonl に足す (途中から再開できる。
設定の順は種ごとに逆)。種は ENOMOTO_T_MIP_SEED=種。計測する wheel は環境変数 TF_PKG (pip install --target の場所) を PYTHONPATH の
先に置いて固定する。stderr の `TREEFIX: summary` の行 (再スタートごとに 1 行) と `MIP: tree restart` の行の数も記録する。
使い方: python run.py <出力.jsonl> <設定,...> <種,...> <時間> <問題 ...|@58|@hit>"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS as P58  # noqa: E402
CFGS = {
    "def": {},
    "tf": {"ENOMOTO_MIP_TREEFIX": "pcur"},
    "tfR": {"ENOMOTO_MIP_TREEFIX": "pcurR"},
    "tfp": {"ENOMOTO_MIP_TREEFIX": "p"},
    "tfpc": {"ENOMOTO_MIP_TREEFIX": "pc"},
}
SETS = {"@58": P58, "@hit": ["csched008", "fiball", "timtab1", "csched007", "neos-957323"]}
out, cfgs, seeds, tl = Path(sys.argv[1]), sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")], int(sys.argv[4])
probs = []
for a in sys.argv[5:]:
    probs += SETS.get(a, [a])
if not out.is_absolute():
    out = D / out
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for seed in seeds:
    cs = cfgs if seed % 2 else cfgs[::-1]
    for p in probs:
        for c in cs:
            if (p, c, seed) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            if os.environ.get("TF_PKG"):
                env["PYTHONPATH"] = os.environ["TF_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **CFGS[c])
            mps = WT / ".miplib_cache" / f"{p}.mps"
            if not mps.exists():
                sys.path.insert(0, str(WT / "scripts"))
                import run_miplib_benchmark as rb  # noqa: E402
                rb.fetch(p, WT / ".miplib_cache")
            t0 = time.time()
            tf, nres = [], 0
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                tf = [l for l in r.stderr.splitlines() if l.startswith("TREEFIX: summary")]
                nres = sum(1 for l in r.stderr.splitlines() if "tree restart:" in l)
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "rep": seed - 1, "tl": tl, "result": res, "treefix": tf, "tree_restarts": nres,
                                    "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
            print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), tf[-1:] , flush=True)
