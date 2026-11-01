"""neos-1171448・neos-1171737 で今の既定 (def) と cs2 を交互に解く (2026-11-01、4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
使い方: python run.py 回数
ログは logs/<問題>_<設定>_r<回>.log (あれば飛ばす)。回ごとに 問題ごとに def → cs2 の順に交互に回す。"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb

PROBS = ["neos-1171448", "neos-1171737"]
CONF = {
    "def": {},
    # analysis/halve_submip_20261030/run.py の CFGS["cs2"] と同じ
    "cs2": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "5", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
}

if __name__ == "__main__":
    reps = int(sys.argv[1])
    L = D / "logs"
    L.mkdir(exist_ok=True)
    for p in PROBS:
        rb.fetch(p, C)
    for k in range(1, reps + 1):
        for p in PROBS:
            for c in ("def", "cs2"):
                out = L / f"{p}_{c}_r{k}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CONF[c])
                t0 = time.time()
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(f"# wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
                print(p, c, out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:200] if r.stdout.strip() else "NO OUTPUT", flush=True)
