"""58 問で `u` (ENOMOTO_MIP_GAP1=u) が働くか (LP の試しで列を固定したか) を、ログつきで短く (既定 30 秒・種 1) 回して見る。
結果は u_hits.jsonl (問題ごとに "MIP: GAP1 u" の行)。使い方: python detect_u.py [時間]"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis" / "implint_tu_20261110"))
import run_miplib_benchmark as rb  # noqa: E402
from probs_all import P58  # noqa: E402
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 30
OUT = D / "u_hits.jsonl"
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
code = ("import sys; from pathlib import Path; from enomoto_solver import benchmark_highs as bh; _, lp = bh._load_lp(Path(sys.argv[1])); "
        "m, _, _ = bh._build_our_model(lp); m.solve(None, False, float(sys.argv[2]), None, None, 4)")
for p in P58:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_GAP1="u")
    try:
        r = subprocess.run([sys.executable, "-c", code, str(mps), str(TL)], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 200)
        lines = [l for l in (r.stderr + r.stdout).splitlines() if "GAP1 u" in l or l.startswith("MIP_FOLD: n ")]
    except Exception as e:  # noqa: BLE001
        lines = ["error " + str(e)[:200]]
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "tl": TL, "lines": lines}) + "\n")
    print(p, lines[-1:] if lines else "-", flush=True)
