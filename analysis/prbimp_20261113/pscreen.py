"""PRBIMP: 前処理だけの見比べ。python pscreen.py 設定 問題... (cfg.py の設定、種 1、4 スレッド)。根の LP の大きさの行が出たら止める (300 秒まで)。
プロービングの行 (MIP: probing: ...) と根の LP を pscreen.jsonl に 1 行ずつ足す。"""
import json, os, re, subprocess, sys, time, threading
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
name = sys.argv[1]
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[name]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_PROBE_IMPLIED="1")
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    res = {"problem": p, "cfg": name, "probe": []}
    killer = threading.Timer(300, pr.kill); killer.start()
    for line in pr.stdout:
        if line.startswith("MIP: probing:") or line.startswith("MIP: presolve: rows"):
            res["probe"].append(line.strip()[:300])
        m = re.match(r"MIP: root LP (-?[\d.e+-]+) \((\d+) iters", line)
        if m: res["root_lp"] = float(m.group(1))
        m = re.match(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", line)
        if m:
            res.update(rows=int(m.group(1)), cols=int(m.group(2)), t=round(time.time() - t0, 2)); pr.kill(); break
        if line.startswith("{"):
            res["final"] = json.loads(line); res["t"] = round(time.time() - t0, 2)
    killer.cancel(); pr.kill(); pr.wait()
    with open(D / "pscreen.jsonl", "a") as f: f.write(json.dumps(res) + "\n")
    print(p, name, res.get("rows"), res.get("cols"), res.get("root_lp"), res.get("t"), flush=True)
