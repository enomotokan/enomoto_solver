"""PRESIZE240: 当方を試作の設定つきで根の LP まで回し、大きさを取る (既存の試作で差が埋まるかを見る)。
使い方: python vary.py 名前 問題[,問題...] [環境変数=値 ...]   出力: vary.jsonl に 1 問 1 行、elog/<問題>.<名前>.log。"""
import json, os, re, subprocess, sys, time, threading
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
name, probs = sys.argv[1], sys.argv[2].split(",")
extra = dict(kv.split("=", 1) for kv in sys.argv[3:])
for p in probs:
    cached = (WT / ".miplib_cache" / f"{p}.mps").exists()
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", **extra)
    t0 = time.time(); r = {"problem": p, "cfg": name, "env": extra}
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "1"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    killer = threading.Timer(630, pr.kill); killer.start()
    with open(D / "elog" / f"{p}.{name}.log", "w") as f:
        for line in pr.stdout:
            f.write(f"{time.time() - t0:8.2f} {line}")
            m = re.match(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+), nonzeros (\d+) -> (\d+)", line)
            if m and "pre1" not in r: r["pre1"] = [int(m.group(i)) for i in (2, 4, 6)]
            m = re.match(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", line)
            if m: r.update(rows=int(m.group(1)), cols=int(m.group(2)), nnz=int(m.group(3)), t=round(time.time() - t0, 2)); pr.kill(); break
    killer.cancel(); pr.kill(); pr.wait()
    with open(D / "vary.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
    print(json.dumps(r), flush=True)
    if not cached: mps.unlink(missing_ok=True)
