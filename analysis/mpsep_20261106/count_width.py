"""最適面の広さ (根の LP の被約費用 0 の非基底の数、`ENOMOTO_MIP_MPSEP_WIDTH_ONLY`) を 58 問 (analysis/cbcimpl_20261029/probs.py) と
Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) で数え、絞った形のつまみ (`g`: 幅 ≥ max(20, 行 × 0.5)、`h`: 根の LP ≥ 0.5 秒 または 行 ≥ 5000)
で働く問題を数える。1 問 1 行を count_width.jsonl に足す (途中から再開できる)。1 問は 4 スレッド、壁時計 `CW_TL` 秒 (既定 120) で打ち切る。
使い方: python3 count_width.py [58|mitt]  (Mittelmann の大きな問題は数えた後で消す)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
which = sys.argv[1] if len(sys.argv) > 1 else "58"
probs = PROBS if which == "58" else sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
OUT = D / "count_width.jsonl"
done = {(json.loads(l)["set"], json.loads(l)["problem"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = float(os.environ.get("CW_TL", "120"))
for p in probs:
    if (which, p) in done:
        continue
    rec = {"set": which, "problem": p}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        rec["error"] = f"fetch: {e}"[:200]
        mps = None
    if mps is not None:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_MPSEP_WIDTH_ONLY="1", ENOMOTO_MIP_THREADS="4")
        t0 = time.time()
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(int(TL)), "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 30)
            ls = [l for l in pr.stderr.splitlines() if l.startswith("MPSEP_WIDTH ")]
            if ls:
                rec.update(json.loads(ls[-1][len("MPSEP_WIDTH "):]))
            else:
                rec["error"] = "no root cut loop (solved or stopped before the cut loop)"
                outs = [l for l in pr.stdout.splitlines() if l.startswith("{")]
                if outs:
                    rec["status"] = json.loads(outs[-1]).get("status")
        except subprocess.TimeoutExpired:
            rec["error"] = "timeout"
        rec["secs"] = round(time.time() - t0, 1)
        if which == "mitt" and mps.stat().st_size > 50e6:
            mps.unlink()
    if "width" in rec:
        rec["wide_g"] = rec["width"] >= max(20, 0.5 * rec["rows"])
        rec["heavy_h"] = rec["root_lp_secs"] >= 0.5 or rec["rows"] >= 5000
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec), flush=True)
