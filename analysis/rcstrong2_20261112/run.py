"""RCSTRONG2 (analysis/rcstrong2_20261112.md): 問題 x 設定 x 種を 4 スレッドで 1 つずつ解き、jsonl に足し、stderr を logs/<出力の名前>.<問題>.<設定>.s<種>.log.gz に残す
(途中から再開できる。ENOMOTO_MIP_LOG=1 で node diag などを出す。解き方は変えない)。計測する wheel は環境変数 RCS_PKG
(pip install --target の場所) を PYTHONPATH の先に置いて固定する。
使い方: python run.py <出力.jsonl> <設定,...> <種,...> <時間> <問題 ...>"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
R = "ENOMOTO_MIP_RCSTRONG"
CFGS = {
    "def": {},
    "irk": {R: "irkc"},
    "i": {R: "ic"},
    "ir": {R: "irc"},
    "k": {R: "kc"},
    "c": {R: "c"},
}
# 直した形 (src/mip/rcstrong.rs の文字 n・g・m): 環境変数 RCS2_EXTRA (名前=値,...) で設定に足すこともできる
for base in ["irk", "i", "ir", "k"]:
    for x in "ngm":
        CFGS[f"{base}{x}"] = {R: CFGS[base][R] + x}
out, cfgs, seeds, tl = Path(sys.argv[1]), sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")], int(sys.argv[4])
probs = sys.argv[5:]
if not out.is_absolute():
    out = D / out
(D / "logs").mkdir(exist_ok=True)
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
KEYS = ["node diag", "adaptive node cuts", "RCSTRONG", "cut pool propagation", "tree search threads"]
for seed in seeds:
    cs = cfgs if seed % 2 else cfgs[::-1]
    for p in probs:
        for c in cs:
            if (p, c, seed) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            if os.environ.get("RCS_PKG"):
                env["PYTHONPATH"] = os.environ["RCS_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", **CFGS.get(c, {}))
            for kv in os.environ.get("RCS2_EXTRA", "").split(","):
                if kv:
                    env.__setitem__(*kv.split("=", 1))
            mps = WT / ".miplib_cache" / f"{p}.mps"
            if not mps.exists():
                import run_miplib_benchmark as rb  # noqa: E402
                rb.fetch(p, WT / ".miplib_cache")
            t0 = time.time()
            diag = []
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                with gzip.open(D / "logs" / f"{out.stem}.{p}.{c}.s{seed}.log.gz", "wt") as g:
                    g.write(r.stderr)
                diag = [l for l in r.stderr.splitlines() if any(k in l for k in KEYS) and "(thread" not in l][-10:]
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "tl": tl, "env": CFGS.get(c, {}), "result": res, "diag": diag,
                                    "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
            nd = [l for l in diag if "node diag" in l]
            print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), (nd[-1][-140:] if nd else ""), flush=True)
