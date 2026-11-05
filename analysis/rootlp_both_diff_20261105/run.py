"""根の LP を二段解法と内点法 + クロスオーバーの両方で最後まで解く診断 (ENOMOTO_MIP_ROOTLP_BOTH_DIAG) を問題ごとに 1 回回す。
使い方: python run.py 出力.jsonl 時間の上限 問題...   (4 スレッド。出力に 1 問 1 行を足す。すでにある問題は飛ばす)"""
import json, os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb
out, tl, probs = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3:]
done = set()
if out.exists():
    done = {json.loads(l)["problem"] for l in out.open()}
for p in probs:
    if p in done:
        continue
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:
        print(p, "fetch failed", e, flush=True)
        continue
    env = dict(os.environ, ENOMOTO_MIP_ROOTLP_BOTH_DIAG="1", ENOMOTO_MIP_THREADS="4")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                           env=env, capture_output=True, text=True, timeout=tl + 120)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
    rec = {"problem": p, "wall": round(time.time() - t0, 2)}
    lines = [l for l in err.splitlines() if l.startswith("ROOTLP_BOTH ")]
    if lines:
        rec.update(json.loads(lines[-1][len("ROOTLP_BOTH "):]))
    else:
        rec["error"] = "no ROOTLP_BOTH line: " + " | ".join(err.strip().splitlines()[-3:])[:400]
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, {k: rec.get(k) for k in ("winner", "t_simplex", "t_i", "ndiff", "frac_s", "frac_i", "frac_both", "basis_diff_cols", "error")}, flush=True)
