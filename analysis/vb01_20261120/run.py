"""SCIP-VB (試作 ENOMOTO_MIP_VB01) の計測の分担を回す: python run.py SET K (SET は P (MIPLIB の 58 問 × 種 1〜3、60 秒) か M (VB01 が働く Mittelmann の問題 × 種 1・2、600 秒))。
shards_SET.json の K 番目の (問題, 設定, 種, 秒) を順に解き、res_SET_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。
行の形は analysis/slowm_20261119/run.py と同じ (この台本はその写し)。設定は cfg.py、どれも 4 スレッド。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
SET, k = sys.argv[1], int(sys.argv[2])
mine = json.loads((D / f"shards_{SET}.json").read_text())["shards"][k]
OUT = D / f"res_{SET}_s{k}.jsonl"
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
def TH(cfg):
    """設定の名前が _t1 で終われば 1 スレッド、scip は 1 (SCIP は既定で逐次。記録だけ)、ほかは 4 スレッド"""
    return "1" if cfg.endswith("_t1") or cfg == "scip" else "4"
def run(cfg, seed, mps, tl):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = TH(cfg); env["ENOMOTO_T_MIP_SEED"] = str(seed)
    env["ENOMOTO_BENCH_SCIP_SEED"] = str(seed)
    t0 = time.time(); peak = 0.0
    py = str(WT / ".v019env/bin/python") if cfg.startswith("v019") else sys.executable  # 0.1.9 は別の venv の python で同じ worker を動かす
    pr = subprocess.Popen([py, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-scip" if cfg == "scip" else "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", TH(cfg)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    out, _ = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1)
for p, cfg, seed, tl in mine:
    if (p, cfg, seed) in done: continue
    for attempt in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e: err = e; time.sleep(30)  # noqa: BLE001
    else:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    res, peak, wall = run(cfg, seed, mps, tl)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), peak, flush=True)
