"""調べ B2: python runB2.py K → res2_K.jsonl と logs/B2_問題_設定.log.gz。K = 問題の番号 × 3 + 組 (0〜2)、9 分担。
1 分担 = 1 問 × 2 設定 (probsB2.CFGS の順に 2 つずつ)、600 秒・種 1、最適値を打ち切り値に与える (整数の目的値は − 0.5)。最大約 21 分。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from probsB2 import PROBS, CFGS  # noqa: E402
k = int(sys.argv[1]); p, opt, integ = PROBS[k // 3]
names = list(CFGS)[2 * (k % 3): 2 * (k % 3) + 2]
cut = opt - 0.5 if integ else opt
OUT = D / f"res2_{k}.jsonl"; LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
done = {json.loads(l)["cfg"] for l in open(OUT)} if OUT.exists() else set()
mps = rb.fetch(p, WT / ".miplib_cache")
for c in names:
    if c in done: continue
    th, extra = CFGS[c]
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(extra); env.update(ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CUTOFF=repr(cut))
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", th],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=1200)
    (LOGS / f"B2_{p}_{c}.log.gz").write_bytes(gzip.compress(r.stderr.encode()))
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "cut": cut, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, c, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
