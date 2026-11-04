"""tree_close2 (2026-11-04): ENOMOTO を 4 スレッドで解き、結果を <out>.jsonl に足す (ログは logs/)。
使い方: python run.py <out> <tag> <秒> <問題,...> [KEY=VAL ...]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
out, tag, tl, probs = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4].split(",")
extra = dict(a.split("=", 1) for a in sys.argv[5:])
(D / "logs").mkdir(exist_ok=True)
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_LOG="1", **extra)
    cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"]
    t0 = time.time()
    try:
        r = subprocess.run(cmd, cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
        stdout, stderr, rc = r.stdout, r.stderr, r.returncode
    except subprocess.TimeoutExpired as e:
        stdout, stderr, rc = "", str(e), -999
    (D / "logs" / f"{out}_{p}_{tag}.log").write_text(stderr[-2000000:] + stdout)
    try:
        res = json.loads([l for l in stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "rc": rc, "time": tl}
    with open(D / f"{out}.jsonl", "a") as f:
        f.write(json.dumps({"problem": p, "tag": tag, "env": extra, "result": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(f"{p:24s} {tag:8s} {res['status']:10s} t={res['time']:7.2f} obj={res.get('objective')} bd={res.get('bound')} root={res.get('root_bound')} nodes={res.get('nodes')}", flush=True)
