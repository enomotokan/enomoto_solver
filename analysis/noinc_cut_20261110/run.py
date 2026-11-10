# 58 問で、最初の暫定解が根のカットのループ (cut round 0) より前に出るかを数える (ENOMOTO_MIP_LOG のログから)
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
TL = sys.argv[1] if len(sys.argv) > 1 else "40"
out = D / "res.jsonl"
done = {json.loads(l)["problem"] for l in out.read_text().splitlines() if l.strip()} if out.exists() else set()
for p in PROBS:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1"})
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
    (D / "logs" / f"{p}.log").write_text(pr.stderr)
    lines = pr.stderr.splitlines()
    t = lambda l: float(re.findall(r"\(([0-9.]+)s\)", l)[-1]) if re.findall(r"\(([0-9.]+)s\)", l) else None
    inc = next(((i, l) for i, l in enumerate(lines) if "MIP: new incumbent" in l), None)
    cut = next(((i, l) for i, l in enumerate(lines) if re.match(r"MIP: cut round 0:.*cuts \(of", l)), None)
    rl = next(((i, l) for i, l in enumerate(lines) if l.startswith("MIP: root LP ") and "iters" in l), None)
    try:
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {}
    r = {"problem": p, "status": res.get("status"), "time": res.get("time"),
         "inc_t": t(inc[1]) if inc else None, "inc_src": re.findall(r"\[(.*)\]", inc[1])[0] if inc and "[" in inc[1] else None,
         "cut0_t": t(cut[1]) if cut else None, "rootlp_t": t(rl[1]) if rl else None,
         "inc_before_cut": (inc[0] < cut[0]) if inc and cut else (inc is not None and cut is None and False) if inc else False,
         "has_cut": cut is not None}
    with out.open("a") as f:
        f.write(json.dumps(r) + "\n")
    print(r, flush=True)
