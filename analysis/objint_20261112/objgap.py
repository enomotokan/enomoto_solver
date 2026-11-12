"""OBJINT の `c` の数え (手順 1): 整数部の刻み s と連続部の幅 M - m を、前処理の直後 (`OBJGAP pre`、ENOMOTO_MIP_DEBUG_IMPLINTCOUNT) と
終わりの大域的な境界 (`OBJGAP end`、ENOMOTO_MIP_DEBUG_OBJGAP) で出す。結果は objgap.jsonl。
使い方: python objgap.py stop [問題 ...]  (前処理の直後で止める) / python objgap.py 秒 [問題 ...]  (その秒だけ解いて終わりの値も出す)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "implint_tu_20261110"))
from probs_all import ALL  # noqa: E402
mode = sys.argv[1]
probs = sys.argv[2:] or ALL + ["csched007"]
OUT = D / ("objgap.jsonl" if mode == "stop" else f"objgap_{mode}s.jsonl")
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
for p in probs:
    if p in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="2", ENOMOTO_MIP_DEBUG_IMPLINTCOUNT="stop" if mode == "stop" else "1", ENOMOTO_MIP_DEBUG_OBJGAP="1")
    tl = 300 if mode == "stop" else int(mode)
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                        "--time-limit", str(tl), "--threads", "2"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 600)
    ls = [l for l in (r.stdout + r.stderr).splitlines() if l.startswith("OBJGAP")]
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "lines": ls, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, ls[:1], ls[-1:] if len(ls) > 1 else "", flush=True)
