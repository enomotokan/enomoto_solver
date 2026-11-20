"""EQUIV2_PWORK の確かめの分担を回す (slow2_20261117/run.py の写し): python run.py SET K (SET は P (60 秒) か M (600 秒))。
shards_SET.json の K 番目の (問題, 設定, 種, 秒) を順に 4 スレッドで解き、res_SET_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。
設定は cfg.py (def = 今の既定、epw = ENOMOTO_MIP_EQUIV2_PWORK=1)。"""
import gzip, json, os, subprocess, sys, tempfile, time
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
LOGS = D / "logs"
DSOL = Path(tempfile.gettempdir()) / "unitcal_7_pre.sol"
if SET.startswith("W") and not DSOL.exists():
    DSOL.write_bytes(gzip.decompress((D / "unitcal_7_pre.sol.gz").read_bytes()))
def wlog(f):
    """ログから MIP_DEBUG_SOL の行 (最初の 200 行) と最後の 60 行を残して gzip にする (全部は大きい)"""
    ls = f.read_text(errors="replace").splitlines()
    dbg = [l for l in ls if "MIP_DEBUG_SOL" in l][:200]
    (f.parent / (f.name + ".gz")).write_bytes(gzip.compress(("\n".join(dbg + ["---- 最後の 60 行 ----"] + ls[-60:]) + "\n").encode()))
    f.unlink()
def run(cfg, seed, mps, tl):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    err = subprocess.DEVNULL
    if SET.startswith("W"):  # 誤答の切り分け: デバッグ解 (前処理の後の問題の最適解) を渡し、標準エラーを logs/ に残す
        env["ENOMOTO_MIP_DEBUG_SOL"] = str(DSOL); env["ENOMOTO_MIP_LOG"] = "1"
        LOGS.mkdir(exist_ok=True); err = open(LOGS / f"{SET}_{cfg}_s{seed}.log", "w")
    t0 = time.time(); peak = 0.0
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs" if cfg == "highs" else "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    out, _ = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    if SET.startswith("W"):
        err.close(); wlog(LOGS / f"{SET}_{cfg}_s{seed}.log")
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
