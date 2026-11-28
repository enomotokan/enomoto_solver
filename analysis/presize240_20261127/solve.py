"""PRESIZE240 7 節: 当方を設定つきで最後まで解き、結果を solve.jsonl に足す。
使い方: python solve.py 名前 問題[,問題...] スレッド 秒 [環境変数=値 ...]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
name, probs, th, tl = sys.argv[1], sys.argv[2].split(","), sys.argv[3], sys.argv[4]
extra = dict(kv.split("=", 1) for kv in sys.argv[5:])
for p in probs:
    cached = (WT / ".miplib_cache" / f"{p}.mps").exists()
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1", **extra)
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) + 300)
    res = next((json.loads(l) for l in reversed(pr.stdout.splitlines()) if l.startswith("{")), {"status": "crash", "err": pr.stderr[-300:]})
    r = {"problem": p, "cfg": name, "threads": th, "env": extra, "result": res, "wall": round(time.time() - t0, 1)}
    with open(D / "solve.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
    print(json.dumps(r), flush=True)
    if not cached: mps.unlink(missing_ok=True)
