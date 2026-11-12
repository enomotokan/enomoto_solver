"""OBJINT の分担を回す: python run.py SET K  (SET は D (診断、600 秒) など。shards_SET.json の K 番目)。
(問題, 設定, 種, 秒) を順に 4 スレッドで解き、res_SET_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。
設定 highs は HiGHS 1.15.1 (random_seed = 種、ログ付き)。SET が D のときはログを logs_D/<問題>.<設定>.s<種>.log.gz に残す。
前もって: sh build.sh (リポジトリの根でリリースのビルドを入れる) と pip install highspy==1.15.1"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS, LOGENV  # noqa: E402
SET, k = sys.argv[1], int(sys.argv[2])
mine = json.loads((D / f"shards_{SET}.json").read_text())["shards"][k]
OUT = D / f"res_{SET}_s{k}.jsonl"
LOGS = SET.startswith("T")
LD = D / f"logs_{SET}"
if LOGS: LD.mkdir(exist_ok=True)
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
def run(cfg, seed, mps, tl, log):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if cfg == "highs":
        kind = "highs"; env["ENOMOTO_BENCH_HIGHS_OPT"] = f"random_seed={seed}"
        if LOGS: env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
    else:
        kind = "ours"; env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
        if LOGS: env.update(LOGENV)
    t0 = time.time(); peak = 0.0; last = [""]; buf = []
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT if LOGS else subprocess.DEVNULL, text=True, errors="replace")
    def pump():
        for line in pr.stdout:
            if line.startswith("{"): last[0] = line
            if LOGS: buf.append(f"{time.time() - t0:8.2f} {line}")
    th = threading.Thread(target=pump); th.start()
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    th.join()
    if LOGS: log.write_bytes(gzip.compress("".join(buf).encode()))  # 終わってからまとめて書く
    try: res = json.loads(last[0])
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
    res, peak, wall = run(cfg, seed, mps, tl, LD / f"{p}.{cfg}.s{seed}.log.gz")
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), peak, flush=True)
