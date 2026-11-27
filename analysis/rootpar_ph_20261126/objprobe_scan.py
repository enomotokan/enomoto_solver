"""OBJPROBE r (カットのループの中の乱択丸め) の回数・秒・見つけた解を数える: python objprobe_scan.py PLAN OUT。4 スレッド・種 1、最初に木に入る所で止める"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
PLAN = json.loads((D / sys.argv[1]).read_text()); OUT = D / sys.argv[2]
EXTRA = dict(a.split("=", 1) for a in os.environ.get("DIAG_EXTRA", "").split() if a)
M = re.compile(r"^MIP: OBJPROBE r: round (\d+): randomized rounding (found a solution|none) \(incumbent (.*?) -> (.*?), ([0-9.]+)s, ([0-9.]+)s\)")
done = set()
if OUT.exists():
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()}
for p, seed, th in PLAN:
    if p in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(EXTRA); env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "120", "--threads", str(th)],
                          cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, errors="replace")
    rows = []; t0 = time.time()
    for l in pr.stderr:
        m = M.match(l)
        if m: rows.append({"round": int(m.group(1)), "found": m.group(2) != "none", "before": m.group(3), "after": m.group(4), "secs": float(m.group(5))})
        if l.startswith("TRSTAT: tree start") or time.time() - t0 > 400: pr.kill(); break
    pr.wait()
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "threads": th, "extra": EXTRA, "rr": rows}) + "\n")
    print(p, len(rows), round(sum(r["secs"] for r in rows), 2), sum(r["found"] for r in rows), flush=True)
