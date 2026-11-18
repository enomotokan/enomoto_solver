"""候補 7 (SYM_LEXGATE) の特徴の走査: P (MIPLIB 58 問) と M (slow2 の 48 問) を、対称性の検出の直後で止めて
(`ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM`)、`ENOMOTO_MIP_SYM_LEXGATE=diag` の特徴の行を拾う。python scan.py 出力"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
S2 = WT / "analysis/slow2_20261117"
names = []
for s in ("P", "M"):
    for sh in json.loads((S2 / f"shards_{s}.json").read_text())["shards"]:
        for x in sh:
            if x[0] not in names:
                names.append(x[0])
out = D / sys.argv[1]
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
for p in names:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SYM_LEXGATE="diag", ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM="1")
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "300", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=400)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    m = re.search(r"LEXGATE .*?\(score ([-\d.]+), sigmoid [\d.]+, (.*)\)", err)
    feats = {k: float(v) for k, v in re.findall(r"(\w+) ([-\d.e+]+)", m.group(2))} if m else None
    rec = {"problem": p, "feats": feats, "lines": [l for l in err.splitlines() if "symmetry" in l][-6:]}
    with open(out, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, feats, flush=True)
