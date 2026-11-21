"""種の揺れの確かめ (2026-11-21): vsscip_20261120 の M で 11/17 より遅く出た comp07-2idx・ran14x18-disj-8 と、
CM2 14 で pe が大きく遅かった fiball を、種を変えて回す。python run.py で res.jsonl に足す (途中から再開できる)。
呼び方は analysis/slowm_20261119/run.py の run() と同じ (scripts/run_miplib_benchmark.py --worker-ours、ENOMOTO_MIP_THREADS=4・ENOMOTO_T_MIP_SEED=s、600 秒)。
- comp07-2idx・ran14x18-disj-8: 今の既定 (def) を種 1〜5
- fiball: {def, pe (ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN=1)} × 種 1〜4"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "pe": {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1"}}
TL = 600
JOBS = [(p, "def", s) for p in ("comp07-2idx", "ran14x18-disj-8") for s in range(1, 6)] + \
       [("fiball", c, s) for s in range(1, 5) for c in ("def", "pe")]
OUT = D / "res.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
def run(cfg, seed, mps):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    t0 = time.time(); peak = 0.0
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > TL + 300: pr.kill(); break
    out, _ = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1)
for p, cfg, seed in JOBS:
    if (p, cfg, seed) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    res, peak, wall = run(cfg, seed, mps)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": TL, "result": res, "max_rss_mb": peak, "wall": wall,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
