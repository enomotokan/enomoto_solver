"""RCSTRONG の計測 (analysis/rcstrong_20261111.md): 問題 x 設定 x 種を 4 スレッドで 1 つずつ解き、jsonl に足す (途中から再開できる。
設定の順は種ごとに逆)。種は ENOMOTO_T_MIP_SEED=種。計測する wheel は環境変数 RCS_PKG (pip install --target の場所) を PYTHONPATH の
先に置いて固定する。stderr の MIP: RCSTRONG の行も記録する。
使い方: python run.py <出力.jsonl> <設定,...> <種,...> <時間> <問題 ...|@58>"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "implint_tu_20261110"))
from probs_all import P58  # noqa: E402
CFGS = {
    "def": {},
    "rcc": {"ENOMOTO_MIP_RCSTRONG": "c"},
    "rci": {"ENOMOTO_MIP_RCSTRONG": "i"},
    "rck": {"ENOMOTO_MIP_RCSTRONG": "k"},
    "rcik": {"ENOMOTO_MIP_RCSTRONG": "ikc"},
    "rcic": {"ENOMOTO_MIP_RCSTRONG": "ic"},
    "rcir": {"ENOMOTO_MIP_RCSTRONG": "irc"},
    "rcirk": {"ENOMOTO_MIP_RCSTRONG": "irkc"},
}
SETS = {"@58": P58}
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
            if os.environ.get("RCS_PKG"):
                env["PYTHONPATH"] = os.environ["RCS_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **CFGS[c])
            mps = WT / ".miplib_cache" / f"{p}.mps"
            if not mps.exists():
                import run_miplib_benchmark as rb  # noqa: E402
                rb.fetch(p, WT / ".miplib_cache")
            t0 = time.time()
            fold = []
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                fold = [l for l in r.stderr.splitlines() if "RCSTRONG" in l][-8:]
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "rep": seed - 1, "tl": tl, "result": res, "rcs": fold, "wall": round(time.time() - t0, 1),
                                    "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
            print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), fold[-1:], flush=True)
