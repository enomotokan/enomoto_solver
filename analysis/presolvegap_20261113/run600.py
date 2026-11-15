"""PRESOLVEGAP (analysis/presolvegap_20261113.md): INTAGG=d が前処理を変える Mittelmann の 7 問を cand1114 (base) と base + ENOMOTO_MIP_TREE2_INTAGG=d (iad) でログ付きで解く。
python run600.py K  (shards600.json の K 番目の (問題, 設定, 種) を順に、600 秒・4 スレッド。res600_sK.jsonl に結果、logs/ にログ (gzip))。
途中から再開できる。ソルバーは同時に 1 つ。wheel は環境変数 FOLD_PKG (pip install --target の場所) があれば PYTHONPATH の先に置く。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cand1114_20261113"))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
LOG = {"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DEBUG_PRETIME": "1"}
CFG = {"base": dict(CAND1114, **LOG), "iad": dict(CAND1114, ENOMOTO_MIP_TREE2_INTAGG="d", **LOG)}
k = int(sys.argv[1])
HV = ""
plan = json.loads((D / "shards600.json").read_text())
TL = plan["tl"]
OUT = D / f"res600_s{k}.jsonl"
(D / "logs").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"], json.loads(l)["seed"]) for l in OUT.read_text().splitlines() if l.strip()}
for p, c, SEED in plan["shards"][k]:
    if (p, c, SEED) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if os.environ.get("FOLD_PKG"):
        env["PYTHONPATH"] = os.environ["FOLD_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(SEED), **CFG[c])
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                        "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 600)
    try:
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": r.returncode}
    with gzip.open(D / "logs" / f"{p}_{c}_s{SEED}.log.gz", "wt") as f:
        f.write(r.stderr)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": SEED, "tl": TL, "result": res, "wall": round(time.time() - t0, 1), "shard": k,
                            "cpu": next((l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name")), ""),
                            "nproc": os.cpu_count(), "highs": HV, "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, c, res.get("status"), res.get("time"), flush=True)
