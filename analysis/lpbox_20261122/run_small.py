"""LPBOX の少数問題の比べ (analysis/lpbox_20261122): python run_small.py PLAN (plan_*.json の (問題, 設定, 種, スレッド, 秒) を順に回す)。
1 回ずつログつきで回し (logs/<問題>_<設定>_s<種>_t<スレッド>.log.gz)、res_small.jsonl に 1 行足す。途中から再開できる。
行: 結果の JSON、最初の暫定解 (時刻・値)、根の終わりの暫定解 (`after root heuristics`)、LPBOX の行 (k・固定・箱・伝播・見つけた数・値・
受け入れ・使った秒)、暫定解の出どころ (`new incumbent ... [ラベル]` の数と最後のラベル)。設定は CFGS。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "lpbox": {"ENOMOTO_MIP_LPBOX": "1"}}
plan = json.loads((D / sys.argv[1]).read_text())
OUT = D / "res_small.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"], r["threads"]))
(D / "logs").mkdir(exist_ok=True)
for p, cfg, seed, th, tl in plan:
    if (p, cfg, seed, th) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=tl + 300)
    wall = round(time.time() - t0, 1)
    log = pr.stderr
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    with gzip.open(D / "logs" / f"{p}_{cfg}_s{seed}_t{th}.log.gz", "wt") as f:
        f.write(log + "\n" + pr.stdout)
    root = re.findall(r"after root heuristics: incumbent (Some\(([-0-9.e+]+)\)|None) \(([0-9.]+)s\)", log)
    lb = re.findall(r"MIP: LPBOX: (k=.*)", log)
    srcs = re.findall(r"new incumbent ([-0-9.e+]+) at node \d+ \(([0-9.]+)s\) \[([^\]]*)\]", log)
    cnt = {}
    for _, _, s in srcs: cnt[s] = cnt.get(s, 0) + 1
    row = {"problem": p, "cfg": cfg, "seed": seed, "threads": th, "tl": tl, "result": res, "wall": wall,
           "root_inc": (float(root[-1][1]) if root[-1][1] else None, float(root[-1][2])) if root else None,
           "lpbox": lb, "lpbox_acc": len(re.findall(r"\[LPBOX\]", log)),
           "src_count": cnt, "last_src": srcs[-1][2] if srcs else None, "at": time.strftime("%H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, cfg, seed, th, res.get("status"), res.get("time"), res.get("first_solution_time"), row["root_inc"], row["lpbox_acc"], row["last_src"], flush=True)
