"""決定的な 1 スレッドの計測の道具 (analysis/timedep_20261114.md の 5・6 節): 4 CPU に 1 本ずつ、4 本同時に解く。
python tool.py OUT.jsonl TAG 問題,... 種,... [--wl 上限.json] [--wlmult x] [--tl 秒] [--log] [--slow] [K=V ...]
- 設定は束 cand1114 + K=V (例 ENOMOTO_MIP_TIMEDEP=mplb)。1 スレッド。
- --wl: 問題ごとの手間の上限 (ENOMOTO_MIP_WORK_LIMIT、外側の求解の LP の反復 + ヒューリスティクスの LP の反復) の JSON {問題: 反復}。
  --wlmult で倍率。上限のある問題は時間の上限を --tl (既定 600 秒) にして安全網にする。上限のない問題は --tl (既定 60 秒)。
- 行: problem, tag, seed, status, objective, nodes, lp_iterations, time, wl。--log なら logs_tool/<TAG>/<問題>_s<種>.log に ENOMOTO_MIP_LOG=1 のログ。
--slow: 同じ CPU で `yes` を回して遅い機械を真似る (壁時計は約半分の速さ、CPU 時間は変わらない)。
済んだ (問題, 種) は飛ばす (OUT に TAG の行があれば)。"""
import json, os, subprocess, sys, threading, queue
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cand1114_20261113"))
from cfg import CAND1114  # noqa: E402
args = sys.argv[1:]
out, tag, probs, seeds = Path(args[0]), args[1], args[2].split(","), [int(s) for s in args[3].split(",")]
opts = {"--wl": None, "--wlmult": "1", "--tl": None}; log = False; slow = False; kv = []
i = 4
while i < len(args):
    a = args[i]
    if a in opts: opts[a] = args[i + 1]; i += 2; continue
    if a == "--log": log = True; i += 1; continue
    if a == "--slow": slow = True; i += 1; continue
    kv.append(a); i += 1
wl = json.loads(Path(opts["--wl"]).read_text()) if opts["--wl"] else {}
mult = float(opts["--wlmult"])
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l)
            if r["tag"] == tag: done.add((r["problem"], r["seed"]))
jobs = queue.Queue()
for s in seeds:
    for p in probs:
        if (p, s) not in done: jobs.put((p, s))
lock = threading.Lock()
CODE = """
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, float(sys.argv[2]), None, None, 1, True); t = time.perf_counter() - t
print("TOOLRES " + json.dumps({k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "lp_iterations", "root_bound")} | {"time": t}), flush=True)
"""
def worker(cpu):
    while True:
        try: p, s = jobs.get_nowait()
        except queue.Empty: return
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CAND1114); env["ENOMOTO_MIP_THREADS"] = "1"; env["ENOMOTO_T_MIP_SEED"] = str(s)
        for x in kv:
            k, v = x.split("=", 1); env[k] = v
        lim = int(wl[p] * mult) if p in wl else None
        if lim: env["ENOMOTO_MIP_WORK_LIMIT"] = str(lim)
        tl = float(opts["--tl"] or (600 if lim else 60))
        if log: env["ENOMOTO_MIP_LOG"] = "1"
        lf = None
        if log:
            (D / "logs_tool" / tag).mkdir(parents=True, exist_ok=True); lf = open(D / "logs_tool" / tag / f"{p}_s{s}.log", "w")
        hog = subprocess.Popen(["taskset", "-c", str(cpu), "yes"], stdout=subprocess.DEVNULL) if slow else None
        pr = subprocess.run(["taskset", "-c", str(cpu), sys.executable, "-c", CODE, str(WT / ".miplib_cache" / f"{p}.mps"), str(tl)],
                            env=env, cwd=WT, stdout=subprocess.PIPE, stderr=lf or subprocess.DEVNULL, text=True)
        if hog: hog.kill(); hog.wait()
        if lf: lf.close()
        res = [l for l in pr.stdout.splitlines() if l.startswith("TOOLRES ")]
        r = json.loads(res[-1][8:]) if res else {"status": "crash"}
        row = {"problem": p, "tag": tag, "seed": s, "wl": lim, "env": kv} | r
        with lock:
            with open(out, "a") as f: f.write(json.dumps(row) + "\n")
            print(p, s, r.get("status"), r.get("nodes"), r.get("lp_iterations"), round(r.get("time") or 0, 1), flush=True)
ts = [threading.Thread(target=worker, args=(c,)) for c in range(4)]
for t in ts: t.start()
for t in ts: t.join()
