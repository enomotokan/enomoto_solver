"""WINWHY: HiGHS に大差で勝つ問題・ENOMOTO だけが解く問題で、HiGHS 1.15.1 と ENOMOTO (既定、種 1) を同じ機械で続けて回し、時刻つきのログを取る。
hiloss_20261111/run.py と同じ作り (ソルバーは同時に 1 つ、4 スレッド)。違いは時間の上限を問題ごとに shards.json で与えることと、
切り分け用に ENOMOTO へ環境変数を足せること。
使い方: python run.py 組 [--env 名前=値,...] [--tag 名] [--only highs|ours] [問題 ...]
  組 = shards.json の "sets" の名前 (各要素は [問題, 秒])。--tag を付けると出力は res_<組>_<tag>.jsonl・logs/<問題>.<tag>.log.gz。
出力: res_<組>.jsonl (1 行 = 問題 x ソルバー)、logs/<問題>.<highs|ours>.log.gz (各行の頭にプロセスの開始からの経過秒)。
ENOMOTO は ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_TREE4_TRACE=10 (経過を出すだけで解き方は変えない)、HiGHS は output_flag=true・mip_rel_gap=0。
前もって: pip install maturin highspy==1.15.1 && pip install -e . (リポジトリの根で。リリースのビルド)"""
import argparse, gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
ap = argparse.ArgumentParser()
ap.add_argument("set"); ap.add_argument("probs", nargs="*")
ap.add_argument("--env", default=""); ap.add_argument("--tag", default=""); ap.add_argument("--only", default="")
ap.add_argument("--seed", default="1")
a = ap.parse_args()
mine = json.loads((D / "shards.json").read_text())["sets"][a.set]
if a.probs: mine = [x for x in mine if x[0] in a.probs]
extra = dict(kv.split("=", 1) for kv in a.env.split(",") if kv)
suffix = f"_{a.tag}" if a.tag else ""
OUT = D / f"res_{a.set}{suffix}.jsonl"
(D / "logs").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["solver"], str(r.get("seed", 1))))
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
def run(solver, mps, log, tl):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if solver == "ours":
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=a.seed, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TREE4_TRACE="10")
        env.update(extra)
    else:
        env.update(ENOMOTO_BENCH_HIGHS_LOG="1")
    t0 = time.time(); peak = 0.0; last = [""]
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{solver}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    def pump():
        with gzip.open(log, "wt") as g:
            for line in pr.stdout:
                if line.startswith("{"): last[0] = line
                g.write(f"{time.time() - t0:8.2f} {line}")
    th = threading.Thread(target=pump); th.start()
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    th.join()
    try: res = json.loads(last[0])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1)
solvers = [a.only] if a.only else ["highs", "ours"]
for p, tl in mine:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for solver in solvers:
        if (p, solver, a.seed) in done: continue
        name = a.tag or solver
        if a.seed != "1": name += f"_s{a.seed}"
        res, peak, wall = run(solver, mps, D / "logs" / f"{p}.{name}.log.gz", tl)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "solver": solver, "seed": int(a.seed), "tl": tl, "env": extra if solver == "ours" else {},
                                "result": res, "max_rss_mb": peak, "wall": wall, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, solver, res.get("status"), res.get("time"), peak, flush=True)
