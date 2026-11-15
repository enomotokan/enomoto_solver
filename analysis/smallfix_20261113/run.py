"""SMALLFIX の確かめの分担を回す: python run.py SET K (SET は P (58 問 60 秒) か H (HIGHSLOSS の 12 問 600 秒))。
shards_SET.json の K 番目の (問題, 設定, 種, 秒) を順に 4 スレッドで解き、res_SET_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。
行の形は analysis/cand1114_20261113/run.py と同じ (+ 木の中の再スタートと clique fallback の数え "cnt")。
どの設定も ENOMOTO_MIP_LOG=1 で回し、標準エラーから再スタート・clique fallback の行だけを logs_SET/<問題>.<設定>.s<種>.txt.gz に残す。"""
import gzip, json, os, re, subprocess, sys, tempfile, time
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
LOGD = D / f"logs_{SET}"
LOGD.mkdir(exist_ok=True)
KEEP = re.compile(r"clique fallback|TRNOFIX|TREE restart|tree restart|TRSTAT|restart check|MIP: restart \d|root LP -?\d|presolve:")
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
def count(lines):
    """木の中の再スタート (実際に行ったもの・固定 0 列のもの) と、TRNOFIX で止めたもの、clique fallback の試しの秒"""
    c = {"tree_rst": 0, "tree_rst_fix0": 0, "trnofix_stop": 0, "clqfb_secs": 0.0, "clqfb_used": 0, "clqfb_capped": 0}
    for l in lines:
        m = re.search(r"MIP: (?:TREE|tree) restart: (\d+) newly fixed", l)
        if m:
            c["tree_rst"] += 1; c["tree_rst_fix0"] += int(m.group(1)) == 0
        if "TRSTAT" in l and "guard stop z" in l:
            c["trnofix_stop"] += 1
        m = re.search(r"clique fallback: .*; (used|not used) \(([\d.]+)s\)", l)
        if m:
            c["clqfb_secs"] += float(m.group(2)); c["clqfb_used"] += m.group(1) == "used"
        if "clique fallback: the trial LP stopped" in l or "the trial skipped" in l:
            c["clqfb_capped"] += 1
    return c
def run(p, cfg, seed, mps, tl):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
    t0 = time.time(); peak = 0.0
    with tempfile.TemporaryFile("w+") as err:
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True)
        while pr.poll() is None:
            peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
            if time.time() - t0 > tl + 300: pr.kill(); break
        out, _ = pr.communicate()
        err.seek(0)
        lines = [l for l in err if KEEP.search(l)]
    with gzip.open(LOGD / f"{p}.{cfg}.s{seed}.txt.gz", "wt") as f:
        f.writelines(lines)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1), count(lines)
for p, cfg, seed, tl in mine:
    if (p, cfg, seed) in done: continue
    for attempt in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e: err = e; time.sleep(30)  # noqa: BLE001
    else:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    res, peak, wall, cnt = run(p, cfg, seed, mps, tl)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k, "cnt": cnt,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), peak, cnt, flush=True)
