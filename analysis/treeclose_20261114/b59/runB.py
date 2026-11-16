"""調べ B (treeclose 9.52 節): 最適解は早く持つのに証明に HiGHS の 5〜10 倍の節点が要る 6 問で、根の下界と木の中の下界の上がり方を並べる。
python runB.py K → res_K.jsonl と logs/ (ours は ENOMOTO_MIP_LOG=1 の標準エラー、highs は HiGHS のログ、どちらも全部を gzip)。
1 分担 = 1 問 × {ours, ours_cut, highs, highs_cut} (600 秒・4 スレッド・種 1)。*_cut は最適値を打ち切り値として与える
(ours: ENOMOTO_MIP_CUTOFF、highs: objective_bound)。整数の目的値の問題は最適値 − 0.5 を与える (暫定解 = 最適値で始めたのと同じ木)。
既定 (cand1118 を既定にした 9ab8b789b 以降) でビルドして回す。途中から再開できる。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
sys.path.insert(0, str(D))
from probsB import PROBS, CFGS  # noqa: E402
k = int(sys.argv[1]); p, opt, integ = PROBS[k]
cut = opt - 0.5 if integ else opt
OUT = D / f"res_{k}.jsonl"; LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
done = {json.loads(l)["cfg"] for l in open(OUT)} if OUT.exists() else set()
mps = rb.fetch(p, WT / ".miplib_cache")
for c in CFGS:
    if c in done: continue
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1")
    hi = c.startswith("highs")
    if hi:
        env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
        if c == "highs_cut": env["ENOMOTO_BENCH_HIGHS_OPT"] = f"objective_bound={cut!r}"
    else:
        env["ENOMOTO_MIP_LOG"] = "1"
        if c == "ours_cut": env["ENOMOTO_MIP_CUTOFF"] = repr(cut)
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs" if hi else "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=1200)
    log = r.stdout if hi else r.stderr
    (LOGS / f"{p}_{c}.log.gz").write_bytes(gzip.compress(log.encode()))
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "cut": cut if c.endswith("_cut") else None, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, c, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
