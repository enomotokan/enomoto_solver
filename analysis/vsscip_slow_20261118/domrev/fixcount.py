"""drev の固定の数を問題ごとに数える (1 スレッド・前処理だけ見る短い回)。python fixcount.py > fixcount.txt
行: 問題 自由な列 (前処理後) 固定の数 割合"""
import json, os, re, subprocess, sys, glob
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
probs = sorted({json.loads(l)["problem"] for f in glob.glob(str(D / "res_*.jsonl")) for l in open(f) if l.strip()})
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_DOMCOL": "1", "ENOMOTO_MIP_DOMCOL_PRED": "1", "ENOMOTO_MIP_DOMCOL_REV": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1",
            "ENOMOTO_MIP_DEBUG_PRETIME": "stop"})
for p in probs:
    sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb
    mps = rb.fetch(p, WT / ".miplib_cache")
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "5", "--threads", "1"],
                           env=env, capture_output=True, text=True, timeout=600)
        log = r.stderr
    except subprocess.TimeoutExpired as e:
        log = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
    m = re.search(r"presolve: rows \d+ -> \d+, free columns \d+ -> (\d+)", log)
    f = re.search(r"presolve: (\d+) dominated binaries fixed", log)
    nfree = int(m.group(1)) if m else -1; nfix = int(f.group(1)) if f else -1
    print(p, nfree, nfix, f"{nfix / nfree:.4f}" if nfree > 0 and nfix >= 0 else "?", flush=True)
