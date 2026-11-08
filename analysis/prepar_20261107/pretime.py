"""前処理の段ごとの秒を測る (analysis/prepar_20261107.md)。ENOMOTO_MIP_DEBUG_PRETIME=stop で根の前で止め、PRETIME の行を読む。
使い方: python pretime.py {p58|mitt} TL OUT.jsonl [追加の環境変数 K=V ...]
途中から再開できる (OUT にある問題は飛ばす)。ソルバーは同時に 1 つ。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
which, TL, OUT = sys.argv[1], sys.argv[2], Path(sys.argv[3])
extra = dict(a.split("=", 1) for a in sys.argv[4:])
if which == "p58":
    from probs import PROBS  # noqa: E402
else:
    sh = json.loads((WT / "analysis/mitt600_20261103/shards2.json").read_text())["shards"]
    PROBS = [p for s in sh for p in s]
only = os.environ.get("PRETIME_ONLY")
if only:
    PROBS = [p for p in PROBS if p in only.split(",")]
done = set()
if OUT.exists():
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()}

def parse(err):
    r = {}
    for l in err.splitlines():
        if l.startswith("PRETIME "):
            for t in l.split()[1:]:
                k, v = t.split("=", 1)
                if "/" in v:
                    s, c = v.split("/"); r[k] = float(s); r[k + "#"] = int(c)
                else:
                    r[k] = float(v)
        elif l.startswith("Presolve:"):
            r["presolve_line"] = l.strip()
    return r

for p in PROBS:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_DEBUG_PRETIME": "stop"})
    env.update(extra)
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 120)
        err, rc = pr.stderr, pr.returncode
    except subprocess.TimeoutExpired as e:
        err, rc = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or ""), "timeout"
    r = {"problem": p, "wall": round(time.time() - t0, 2), "rc": rc, **parse(err)}
    if "total" not in r:
        r["tail"] = err[-400:]
    with OUT.open("a") as f:
        f.write(json.dumps(r) + "\n")
    print(p, r.get("total"), r["wall"], flush=True)
