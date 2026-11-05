"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (base) と試作で 1 回ずつ回す。
使い方: python run58.py 出力.jsonl 設定名...   (設定は CFGS のキー。問題ごとに設定の順を交互に逆にする)"""
import json, os, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"base": {}, "scale": {"ENOMOTO_SIMPLEX_SCALE": "1"}, "scale1k": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "1000"},
        "comb": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r"},
        "combB": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r",
                  "ENOMOTO_SIMPLEX_NOISE_RESET": "0.1", "ENOMOTO_SIMPLEX_NOISE_RESET_W": "e"}}
TL, TH = 60, 4
out = Path(sys.argv[1])
names = sys.argv[2:]
probs = os.environ.get("RUN58_PROBS", "").split() or PROBS
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"]))
for k, p in enumerate(probs):
    for cfg in (names if k % 2 == 0 else names[::-1]):
        if (p, cfg) in done:
            continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = dict(os.environ, **CFGS[cfg])
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", str(TH)],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(out, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "rep": 0, "wall": time.time() - t0, "result": res}) + "\n")
        print(p, cfg, res.get("status"), res.get("time"), res.get("objective"), flush=True)
