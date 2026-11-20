"""診断 ENOMOTO_MIP_DIAG_OBBT (README §15) を scan_d.jsonl の 90 問 (P 58・M 32) にかける。U は res_*.jsonl の最適値 (解けた回)、解けなかった問題は
いちばん良い暫定解 (bound と比べて向きを決める)。1 スレッドで解き (根のカットの各ラウンドの被約費用を溜める ENOMOTO_MIP_TREE4_RCUNION=1 つき)、根で OBBTLAM・OBBTDIAG の行が出たら止める (出なければ 900 秒で打ち切る)。→ scan_obbt.jsonl"""
import glob, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
import re
def jl(s):
    # 空の種類の L の範囲は inf・NaN になるので null に直して読む
    return json.loads(re.sub(r"-?\binf\b|\bNaN\b", "null", s))
best = {}
for f in glob.glob(str(D / "res_*.jsonl")):
    for l in open(f):
        r = json.loads(l); res = r.get("result") or {}
        z, b = res.get("objective"), res.get("bound")
        if z is None:
            continue
        opt = res.get("status") == "optimal"
        cur = best.get(r["problem"])
        if opt:
            best[r["problem"]] = (z, True)
        elif cur is None or not cur[1]:
            mx = b is not None and b > z  # 最大化
            if cur is None or (z > cur[0] if mx else z < cur[0]):
                best[r["problem"]] = (z, False)
done = set()
OUT = D / "scan_obbt.jsonl"
if OUT.exists():
    done = {json.loads(l)["problem"] for l in open(OUT)}
for l in open(D / "scan_d.jsonl"):
    r = json.loads(l); p = r["problem"]
    if p in done or p not in best:
        continue
    u, opt = best[p]
    env = {k: x for k, x in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_DIAG_OBBT=repr(u), ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TREE4_RCUNION="1")
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", "900", "--threads", "1"],
                          cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    t0 = time.time(); rec = None; lam = None
    for line in pr.stderr:
        if line.startswith("OBBTLAM "):
            lam = jl(line.split(" ", 1)[1])
        if line.startswith("OBBTDIAG "):
            rec = jl(line.split(" ", 1)[1]); break
    pr.kill(); pr.wait()
    out = {"problem": p, "set": r["set"], "u_opt": opt, "diag": rec, "lam": lam, "wall": round(time.time() - t0, 1)}
    with open(OUT, "a") as f:
        f.write(json.dumps(out) + "\n")
    print(p, opt, rec and {k: rec[k] for k in ("free_int", "rc", "objprop", "sample", "done", "obbt", "sample_rc_or_obj")}, flush=True)
