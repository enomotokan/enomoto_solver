"""neos-1171448 で今の既定 (def) と trim3 を交互に解く (2026-11-01、4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
使い方: python run.py 回数 [v]  (v をつけると ENOMOTO_MIP_SUBMIP_VERBOSE=1 で、ログは logs_v/)
ログは logs[_v]/<設定>_r<回>.log (あれば飛ばす)。回ごとに def → trim3 の順に交互に回す。"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
C = WT / ".miplib_cache"
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb

P = "neos-1171448"
CONF = {
    "def": {},
    # analysis/halve_submip_20261030/run.py の CFGS["trim3"] と同じ
    "trim3": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_RESUME": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
}

if __name__ == "__main__":
    reps = int(sys.argv[1])
    verbose = len(sys.argv) > 2 and sys.argv[2] == "v"
    L = D / ("logs_v" if verbose else "logs")
    L.mkdir(exist_ok=True)
    rb.fetch(P, C)
    for k in range(1, reps + 1):
        for c in ("def", "trim3"):
            out = L / f"{c}_r{k}.log"
            if out.exists():
                continue
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CONF[c])
            if verbose:
                env["ENOMOTO_MIP_SUBMIP_VERBOSE"] = "1"
            t0 = time.time()
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{P}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            out.write_text(f"# wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
            print(c, out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:200] if r.stdout.strip() else "NO OUTPUT", flush=True)
