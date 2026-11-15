"""診断 ENOMOTO_MIP_PROBE_BASIS_DIAG=1 で、根の LP を列だけの基底からと基底なし (同時実行) で解き比べる (4 スレッド、--time-limit 300、
1 回の求解は ENOMOTO_T_MIP_PROBE_BASIS_DIAG_SECS=140 秒まで)。結果は diag.jsonl に足す。
使い方: [DIAG_MODE=wdcf] [DIAG_OUT=diag_v2.jsonl] python diag.py [問題 ...] (なければ probs.HEAVY)
DIAG_MODE の文字: w 列だけの基底 (既定で行つきの問題は W のときだけ)、d 既定どおりの基底 (行つき)、c 基底なし、f 行の中身で作り直した基底"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from probs import HEAVY  # noqa: E402
MODE = os.environ.get("DIAG_MODE", "1")
OUT = D / os.environ.get("DIAG_OUT", "diag.jsonl")
done = set()
if OUT.exists():
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()}
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
KEYS = ("LP probing: root LP", "the first LP stopped", "root LP basis from", "clique", "PROBE_BASIS")
(D / "logs").mkdir(exist_ok=True)
for p in (sys.argv[1:] or HEAVY):
    if p in done:
        continue
    env = dict(base, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_PROBE_BASIS_DIAG=MODE, ENOMOTO_T_MIP_PROBE_BASIS_DIAG_SECS="140")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                            "--time-limit", "300", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=900)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
    (D / "logs" / f"{OUT.stem}_{p}.log").write_text(err)
    pb = [l for l in err.splitlines() if l.startswith("PROBE_BASIS ")]
    rec = {"problem": p, "diag": json.loads(pb[-1][12:]) if pb else None,
           "notes": [l[:240] for l in err.splitlines() if any(k in l for k in KEYS[:3])][:6], "wall": round(time.time() - t0, 1)}
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    d = rec["diag"] or {}
    print(p, json.dumps(d)[:400], flush=True)
