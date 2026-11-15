"""ENOMOTO_MIP_DEBUG_GCDCOUNT=stop で、前処理の丸めの直前 (前処理の呼び出しごと) と前処理の最後の問題の数を集める (2026-11-07)。
1 問ずつ worker を起こし、"GCDCOUNT final" が出たらプロセスが終わる。前処理が終わらない問題は WALL 秒で止める。
使い方: python count.py out.jsonl [set ...] (set は gprobs.SETS の名前、既定は全部)。環境変数を足すと (例 ENOMOTO_MIP_GCDFORCE=1) そのまま渡る"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
import gprobs
out = Path(sys.argv[1])
sets = sys.argv[2:] or list(gprobs.SETS)
WALL = float(os.environ.get("GC_WALL", "300"))
done = set()
if out.exists():
    done = {json.loads(l)["problem"] for l in out.open()}
probs = list(dict.fromkeys(p for s in sets for p in gprobs.SETS[s]))
num = r"\[(\d+), (\d+), (\d+), (\d+)\]"
pat = re.compile(r"GCDCOUNT (presolve depth \d+|final: rows \d+|final_impl: rows \d+):? ineq (\d+) g>1 (\d+) raw uniq/red/infeas/force " + num + " rounded " + num + " only_rounded " + num + r" eq_infeas (\d+)")
for p in probs:
    if p in done:
        continue
    mps = WT / ".miplib_cache" / f"{p}.mps"
    if not mps.exists():
        print(p, "missing", flush=True)
        continue
    env = dict(os.environ, ENOMOTO_MIP_DEBUG_GCDCOUNT="stop", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "100000", "--threads", "4"],
                           capture_output=True, env=env, text=True, timeout=WALL)
        err, to = r.stderr, False
    except subprocess.TimeoutExpired as e:
        err, to = (e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr or "")), True
    rec = {"problem": p, "timeout": to, "calls": [], "final": None, "log": []}
    for line in err.splitlines():
        m = pat.search(line)
        if m:
            g = [int(x) for x in m.groups()[1:]]
            d = {"ineq": g[0], "g_gt1": g[1], "raw": g[2:6], "rnd": g[6:10], "only_rnd": g[10:14], "eq_infeas": g[14]}
            if m.group(1).startswith("final"):
                d["rows"] = int(m.group(1).split()[-1])
                rec["final_impl" if m.group(1).startswith("final_impl") else "final"] = d
            else:
                d["tag"] = m.group(1)
                rec["calls"].append(d)
        elif "GCDFORCE" in line or "HNFPRE" in line or "infeasible" in line.lower():
            rec["log"].append(line.strip()[:200])
    rec["wall"] = round(time.time() - t0, 1)
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps({k: rec[k] for k in ("problem", "timeout", "final", "wall")}), flush=True)
