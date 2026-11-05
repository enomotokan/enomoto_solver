"""根の LP (前処理・LP プロービングの後、カットの前) を二段解法だけで解く診断 (ENOMOTO_MIP_ROOTLP_SIMPLEX_DIAG) を問題ごとに 1 回回す。
使い方: python run.py 出力.jsonl 時間の上限 問題...   (ログは logs/<出力の名前>/<問題>.log、LP は lp/<問題>.mps)
環境変数はそのまま渡す (試作の ENOMOTO_SIMPLEX_... など)。DUMP=1 なら書き出すだけ (解かない)。"""
import json, os, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb
out, tl, probs = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3:]
logd = HERE / "logs" / out.stem
logd.mkdir(parents=True, exist_ok=True)
lpd = Path(os.environ.get("LPDIR", "/tmp/rootlp_mps"))
lpd.mkdir(parents=True, exist_ok=True)
done = set()
if out.exists():
    done = {json.loads(l)["problem"] for l in out.open()}
for p in probs:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(os.environ, ENOMOTO_MIP_ROOTLP_SIMPLEX_DIAG=str(lpd / p), ENOMOTO_MIP_THREADS="4")
    if os.environ.get("DUMP"):
        env["ENOMOTO_MIP_ROOTLP_DUMP_ONLY"] = "1"
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                           env=env, capture_output=True, text=True, timeout=tl + 300)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
    (logd / f"{p}.log").write_text(err[-2_000_000:])
    rec = {"problem": p, "wall": round(time.time() - t0, 2)}
    lines = [l for l in err.splitlines() if l.startswith("ROOTLP_SIMPLEX ")]
    if lines:
        rec.update(json.loads(lines[-1][len("ROOTLP_SIMPLEX "):]))
    else:
        rec["error"] = "no ROOTLP_SIMPLEX line: " + " | ".join(err.strip().splitlines()[-3:])[:400]
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, {k: rec.get(k) for k in ("m", "n", "st", "t", "iters", "polish", "obj", "error")}, flush=True)
