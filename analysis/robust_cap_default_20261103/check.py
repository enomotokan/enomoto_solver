"""p (ENOMOTO_MIP_CUT_LP_CAP_ROBUST) の既定化の確かめ (2026-11-02): 新しいビルドで、環境変数なし (new)・flbgx の環境変数を明示 (new_flbgx)・元に戻す環境変数 (revert) を
同じ問題で解き、結果と、早めの探索・写しの列の値が働いたかのログの行を並べる。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CD = {"ENOMOTO_MIP_CUT_LP_CAP_ROBUST": "p"}
REV = {k: "0" for k in CD}
CFGS = {"new": {}, "new_cd": CD, "revert": REV}
for p in sys.argv[1].split(","):
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CFGS.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **e)
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        log = r.stderr + r.stdout
        (D / "logs" / f"{p}_{c}.log").write_text(log)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        early = [l.strip() for l in log.splitlines() if "progress" in l.lower() and "cut" in l.lower()][:1]
        print(f"{p:20s} {c:10s} {res['status']:10s} {res['time']:6.1f}s obj={res.get('objective')} early={early}", flush=True)
