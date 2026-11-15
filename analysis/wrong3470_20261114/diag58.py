"""58 問 (analysis/rcstrong2_20261112/shards_P.json の問題) を既定で 4 スレッド・TL 秒ずつ解き、直しが働いた回数を数える
(python3 diag58.py [TL=20])。ENOMOTO_DEBUG_EMPTY_ROW (空行を下限 1e-7 で初めて通した) と ENOMOTO_DEBUG_INT_DOUBLETON_SNAP
(整数どうしの二項等式の丸めで値が変わった) の出力の行数を問題ごとに diag58.jsonl に書く。途中から再開できる。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 20.0
ps = sorted({t[0] for sh in json.loads((WT / "analysis/rcstrong2_20261112/shards_P.json").read_text())["shards"] for t in sh})
OUT = D / "diag58.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in ps:
    if p in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_DEBUG_EMPTY_ROW="1", ENOMOTO_DEBUG_INT_DOUBLETON_SNAP="1", ENOMOTO_MIP_THREADS="4")
    pr = subprocess.run([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                        cwd=WT, env=env, capture_output=True, text=True, errors="replace", timeout=TL + 600)
    lines = (pr.stdout + pr.stderr).splitlines()
    e = [l for l in lines if l.startswith("EMPTY_ROW:")]; s = [l for l in lines if l.startswith("INT_DOUBLETON_SNAP:")]
    res = next((json.loads(l) for l in reversed(pr.stdout.splitlines()) if l.startswith("{")), {})
    rec = {"problem": p, "empty_row": len(e), "snap": len(s), "ex": (e + s)[:3], "status": res.get("status"), "objective": res.get("objective")}
    print(rec, flush=True)
    with OUT.open("a") as f: f.write(json.dumps(rec) + "\n")
