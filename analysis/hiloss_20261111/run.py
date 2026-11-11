"""HILOSS: 負けている問題で HiGHS と ENOMOTO (今の既定、種 1) を同じ機械で続けて回し、ログを取る。
使い方: python run.py K [問題]   (shards.json の K 番目の分担、問題を書けばその 1 問だけ。途中から再開できる。ソルバーは同時に 1 つ、4 スレッド・600 秒)
出力: res_sK.jsonl (1 行 = 問題 x ソルバー)、logs/<問題>.<highs|ours>.log.gz (各行の頭に、プロセスの開始からの経過秒を付ける)。
ENOMOTO は ENOMOTO_MIP_LOG=1 (経過を標準エラーに出すだけで解き方は変えない)、HiGHS は output_flag=true・mip_rel_gap=0。
前もって: pip install maturin highspy && pip install -e . (リポジトリの根で。リリースのビルド)"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = 600
k = int(sys.argv[1])
mine = json.loads((D / "shards.json").read_text())["shards"][k]
if len(sys.argv) > 2: mine = [sys.argv[2]]
OUT = D / f"res_s{k}.jsonl"
(D / "logs").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["solver"]))
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
def run(solver, mps, log):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if solver == "ours":
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
    else:
        env.update(ENOMOTO_BENCH_HIGHS_LOG="1")
    t0 = time.time(); peak = 0.0; last = [""]
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{solver}", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    def pump():
        with gzip.open(log, "wt") as g:
            for line in pr.stdout:
                if line.startswith("{"): last[0] = line
                g.write(f"{time.time() - t0:8.2f} {line}")
    th = threading.Thread(target=pump); th.start()
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > TL + 300: pr.kill(); break
    th.join()
    try: res = json.loads(last[0])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1)
for p in mine:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for solver in ("highs", "ours"):
        if (p, solver) in done: continue
        res, peak, wall = run(solver, mps, D / "logs" / f"{p}.{solver}.log.gz")
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "solver": solver, "seed": 1, "tl": TL, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k,
                                "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, solver, res.get("status"), res.get("time"), peak, flush=True)
