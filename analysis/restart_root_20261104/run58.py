"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (def) と試作 (rr = ENOMOTO_MIP_RESTART_ROOT_CUTS=1) で 1 回ずつ
解き、res58.jsonl に書く。問題ごとに設定の順を入れ替える。どちらも詳細ログ (ENOMOTO_MIP_LOG=1) をつけ、summ_log.py の要約を logs58/ に置く。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "rr": {"ENOMOTO_MIP_RESTART_ROOT_CUTS": "1"}}
names = sys.argv[1:] or ["def", "rr"]
OUT = D / "res58.jsonl"
(D / "logs58").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(OUT)}
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
for i, p in enumerate(PROBS):
    mps = rb.fetch(p, WT / ".miplib_cache")
    order = names if i % 2 == 0 else names[::-1]
    for c in order:
        if (p, c) in done:
            continue
        env = dict(base, ENOMOTO_MIP_LOG="1", **CFGS[c])
        t0 = time.time()
        q = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                           env=env, capture_output=True, text=True, timeout=400)
        try:
            r = json.loads(q.stdout.strip().splitlines()[-1])
        except Exception:
            r = {"status": "error", "stderr": q.stderr[-500:]}
        r["mip_warnings"] = q.stderr.count("MIP: WARNING")
        errf = D / "logs58" / f"{c}_{p}.err"
        errf.write_text(q.stderr)
        s = subprocess.run([sys.executable, str(D / "summ_log.py"), str(errf)], capture_output=True, text=True).stdout
        (D / "logs58" / f"{c}_{p}.txt").write_text(s)
        errf.unlink()
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "result": r, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, r.get("status"), r.get("time"), r.get("objective"), r.get("bound"), flush=True)
