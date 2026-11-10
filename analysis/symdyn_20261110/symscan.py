"""対称性の検出だけを回す走査 (ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM): 58 問と 47 問で、orbital fixing の生成元・オービトープ・
SYMREFL の反転の生成元の数を拾う。python symscan.py 出力 (問題の並びは probs.py の 58 問 + shards8.json の 47 問)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
import probs  # noqa: E402
P47 = json.loads((WT / "analysis/mitt600_20261103/shards8.json").read_text())["problems"]
out = D / sys.argv[1]
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
for p in probs.PROBS + [q for q in P47 if q not in probs.PROBS]:
    if p in done:
        continue
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        print(p, "fetch fail", e, flush=True); continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SYMREFL="1", ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM="1")
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "300", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=400)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    lines = [l for l in err.splitlines() if "symmetry" in l or "SYMREFL" in l]
    g = lambda pat: (lambda m: m.group(1) if m else None)(re.search(pat, err))
    rec = {"problem": p, "in58": p in probs.PROBS, "in47": p in P47, "of": g(r"orbital fixing with (\d+) of"), "gens": g(r"symmetry: (\d+) generators"),
           "orbitopes": g(r"(\d+) full orbitopes"), "refl": g(r"SYMREFL (\d+) reflection generators"), "refl_kept": g(r"SYMREFL keeps (\d+) of"), "lines": lines[-8:]}
    with open(out, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, rec["gens"], rec["of"], rec["orbitopes"], rec["refl"], rec["refl_kept"], flush=True)
