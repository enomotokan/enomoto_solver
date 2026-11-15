"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、指定の設定で 1 回ずつ解く (2026-11-06、S3)。
使い方: python run58.py タグ 回 [名前=値 ...] [--only 問題,問題]
ENOMOTO_MIP_LOG=1・ENOMOTO_T_MIP_SEED=回。ログは /tmp/s3_live58 に書き、要点 (試作の行・結果) を b58/<タグ>_<回>.jsonl に足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
args = sys.argv[1:]
only = None
if "--only" in args:
    i = args.index("--only"); only = args[i + 1].split(","); del args[i:i + 2]
tag, rep = args[0], args[1]
extra = dict(a.split("=", 1) for a in args[2:])
OUT = D / "b58" / f"{tag}_{rep}.jsonl"
done = set()
if OUT.exists():
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()}
LIVE = Path("/tmp/s3_live58"); LIVE.mkdir(exist_ok=True)
for p in PROBS:
    if p in done or (only and p not in only):
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=rep, **extra)
    log = LIVE / f"{p}_{tag}_{rep}.log"
    t0 = time.time()
    with open(log, "w") as lf:
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True, timeout=400)
            out = pr.stdout
        except subprocess.TimeoutExpired:
            out = ""
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash"}
    hits = [l.strip()[:200] for l in open(log, errors="replace") if "S3_" in l]
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "tag": tag, "rep": rep, "env": extra, "result": res, "s3_lines": hits, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, tag, rep, res.get("status"), res.get("time"), len(hits), flush=True)
