"""res9 (2026-11-11): Mittelmann 186 問 (plan9.py) を、今の既定 ("ours"、環境変数なし) x 種 1・2 で
4 スレッド・600 秒で 1 回ずつ解き、res9_sK.jsonl に足す。分担表 shards9.json の K 番目の (問題, 設定, 種) の並びを順に回す。途中から再開できる。
ソルバーは同時に 1 つ。HiGHS は res_s*.jsonl の 1 回を使う。行の形は run600.py と同じ (+ "seed")。
使い方: python analysis/mitt600_20261103/run9.py K"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "halve_submip_20261030"))
import run_miplib_benchmark as rb  # noqa: E402
from run import CFGS  # noqa: E402
TL = int(os.environ.get("MITT600_TL", "600"))
k = int(sys.argv[1])
mine = json.loads((D / "shards9.json").read_text())["shards"][k]
OUT = D / f"res9_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["solver"], r["seed"]))

def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0

def run(solver, seed, mps):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if ":" in solver:
        env.update(CFGS[solver.split(":", 1)[1]])
    env["ENOMOTO_MIP_THREADS"] = "4"
    env["ENOMOTO_T_MIP_SEED"] = str(seed)
    t0 = time.time(); peak = 0.0
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps),
                           "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(1)
        if time.time() - t0 > TL + 300:
            pr.kill(); break
    out, _ = pr.communicate()
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    return res, round(peak), round(time.time() - t0, 1)

for p, solver, seed in mine:
    if (p, solver, seed) in done:
        continue
    for attempt in range(5):
        try:
            mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e:  # noqa: BLE001
            err = e; time.sleep(30)
    else:
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "solver": solver, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    res, peak, wall = run(solver, seed, mps)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "solver": solver, "seed": seed, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, solver, seed, res.get("status"), res.get("time"), peak, flush=True)
