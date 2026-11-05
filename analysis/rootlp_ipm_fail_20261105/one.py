"""根の LP (前処理・LP プロービングの後、カットの前) を内点法 + クロスオーバーだけで 1 回解く (ENOMOTO_MIP_ROOTLP_BOTH_DIAG +
ENOMOTO_MIP_ROOTLP_IPM_ONLY)。stderr 全体を logs/<問題><接尾辞>.log に残し、ROOTLP_BOTH の JSON を 1 行で出力に足す。
使い方: python one.py 出力.jsonl 接尾辞 時間の上限 問題... [--dump] [--both]   (環境変数はそのまま子に渡る。4 スレッド)
  --dump: 根の LP を mps/<問題>.mps に書き出す (ENOMOTO_MIP_ROOTLP_DUMP)。--both: 二段解法も走らせる (IPM_ONLY なし)。"""
import json, os, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb
args = [a for a in sys.argv[1:] if not a.startswith("--")]
flags = {a for a in sys.argv[1:] if a.startswith("--")}
out, suf, tl, probs = Path(args[0]), args[1], float(args[2]), args[3:]
(HERE / "logs").mkdir(exist_ok=True)
(HERE / "mps").mkdir(exist_ok=True)
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(os.environ, ENOMOTO_MIP_ROOTLP_BOTH_DIAG="1", ENOMOTO_MIP_THREADS="4")
    if "--both" not in flags:
        env["ENOMOTO_MIP_ROOTLP_IPM_ONLY"] = "1"
    if "--dump" in flags:
        env["ENOMOTO_MIP_ROOTLP_DUMP"] = str(HERE / "mps" / f"{p}.mps")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                           env=env, capture_output=True, text=True, timeout=tl + 300)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
    (HERE / "logs" / f"{p}{suf}.log").write_text(err)
    rec = {"problem": p, "suffix": suf, "wall": round(time.time() - t0, 2)}
    lines = [l for l in err.splitlines() if l.startswith("ROOTLP_BOTH ")]
    if lines:
        rec.update(json.loads(lines[-1][len("ROOTLP_BOTH "):]))
    else:
        rec["error"] = "no ROOTLP_BOTH line: " + " | ".join(err.strip().splitlines()[-3:])[:400]
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, {k: rec.get(k) for k in ("st_s", "st_i", "obj_i", "t_ipm", "t_xo", "t_i", "t_presolve", "error")}, flush=True)
