"""M (Mittelmann MIP 48 問) の分担を回す: python runM.py K (shards_M.json の K 番目を順に 4 スレッド・600 秒で解き、res_M_sK.jsonl に足す。
途中から再開できる。ソルバーは同時に 1 つ)。行の形は analysis/treeclose_20261114/b58v/run.py と同じ。比べは python ci58.py 600 plain nbg res_M_s*.jsonl
(ci58.py は回の名前に rep を使うので、ここでは種を rep として書く)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"plain": {}, "nbg": {"ENOMOTO_MIP_NODELP_BFRT_GROUP": "1"}}
k = int(sys.argv[1])
mine = json.loads((D / "shards_M.json").read_text())["shards"][k]
OUT = D / f"res_M_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
for p, cfg, seed, tl in mine:
    if (p, cfg, seed) in done: continue
    for attempt in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e: err = e; time.sleep(30)  # noqa: BLE001
    else:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "rep": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    t0 = time.time(); peak = 0.0
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    out, _ = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "rep": seed, "tl": tl, "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1), "shard": k}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), round(peak), flush=True)
