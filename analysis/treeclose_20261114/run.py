"""TREECLOSE (analysis/treeclose_20261114.md) の診断の計測: HiGHS だけが解く「木を閉じきれない」型の問題を、cand1114
(analysis/cand1114_20261113/cfg.py) + ログ (ENOMOTO_MIP_LOG・ENOMOTO_PROF_TREE_PAR・ENOMOTO_MIP_DEBUG_PRETIME・ENOMOTO_MIP_TCTRACE、解く側は変えない) と
HiGHS 1.15.1 (ログ付き) で 600 秒・4 スレッド・種 1 で解く。
python run.py K [plan.json]  (plan の shards[K] の (問題, 設定) を順に。res_<plan名>_sK.jsonl に結果、logs/<問題>_<設定>.log.gz にログ)。
途中から再開できる。ソルバーは同時に 1 つ。設定: cand = cand1114 + ログ、highs = HiGHS、ほかは plan の "cfgs" (cand1114 に足す環境変数。名前が highs で始まるものは
HiGHS の回に足す環境変数、例えば ENOMOTO_BENCH_HIGHS_OPT)。分担表の 1 項目は [問題, 設定, 種 (省略可), この回だけ足す環境変数 (省略可)]。
環境変数 FOLD_PKG (pip install --target の場所) があれば PYTHONPATH の先に置く。HiGHS は 1.15.1 でなければ止まる。"""
import gzip, json, os, resource, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cand1114_20261113"))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
LOG = {"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DEBUG_PRETIME": "1", "ENOMOTO_PROF_TREE_PAR": "1", "ENOMOTO_MIP_TCTRACE": "1"}
k = int(sys.argv[1])
pf = D / (sys.argv[2] if len(sys.argv) > 2 else "plan.json")
plan = json.loads(pf.read_text())
CFG = {"cand": dict(CAND1114, **LOG)}
for name, ex in plan.get("cfgs", {}).items():
    if not name.startswith("highs"):
        CFG[name] = dict(CAND1114, **LOG, **ex)
TL, SEED = plan["tl"], plan.get("seed", 1)
OUT = D / f"res_{pf.stem}_s{k}.jsonl"
(D / "logs").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"], json.loads(l)["seed"]) for l in OUT.read_text().splitlines() if l.strip()}
HV = ""
for item in plan["shards"][k]:
    p, c = item[0], item[1]
    seed = item[2] if len(item) > 2 else SEED
    extra = item[3] if len(item) > 3 else {}  # この回だけ足す環境変数 (問題ごとの打ち切り値など)
    if (p, c, seed) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if os.environ.get("FOLD_PKG"):
        env["PYTHONPATH"] = os.environ["FOLD_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
    if c.startswith("highs"):
        HV = subprocess.run([sys.executable, "-c", "import highspy;h=highspy.Highs();print(h.version())"], capture_output=True, text=True, env=env).stdout.strip()
        if not HV.startswith("1.15.1"):
            sys.exit(f"HiGHS {HV!r} (1.15.1 が要る)")
        env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
        env.update(plan.get("cfgs", {}).get(c, {}), **extra)
        kind = "--worker-highs"
    else:
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **CFG[c])
        env.update(extra)
        kind = "--worker-ours"
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), kind, str(mps),
                        "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 900)
    try:
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": r.returncode}
    tag = f"{p}_{c}" + (f"_s{seed}" if seed != 1 else "")
    with gzip.open(D / "logs" / f"{tag}.log.gz", "wt") as f:
        f.write(r.stdout if c.startswith("highs") else "")
        f.write(r.stderr)
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss // 1024  # 子の最大 (それまでの回を含む)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "tl": TL, "result": res, "max_rss_mb": rss, "wall": round(time.time() - t0, 1), "shard": k,
                            "cpu": next((l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name")), ""),
                            "nproc": os.cpu_count(), "highs": HV if c.startswith("highs") else "", "extra": extra, "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, c, seed, res.get("status"), res.get("time"), flush=True)
