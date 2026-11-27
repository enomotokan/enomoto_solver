"""cond の判断だけを見る: 各問題を 1 スレッド・短い時間で回し、ログの「ORBPP cond:」の行を拾う。python decide.py [TL [問題...]] > decide_v3.txt (既定 TL 5 秒、問題は 15 問)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = sys.argv[1] if len(sys.argv) > 1 else "5"
probs = sys.argv[2:] or ["neos-911970", "misc07"] + [c["problem"] for c in json.loads((D / "cands.json").read_text())]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_ORBPP": "cond", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1"})
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    try:
        err = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "1"],
                             cwd=WT, env=env, capture_output=True, text=True, timeout=600).stderr
    except subprocess.TimeoutExpired as e:
        err = e.stderr.decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    m = re.findall(r"ORBPP cond: .*", err)
    print(p, m[0] if m else "(行なし)", flush=True)
