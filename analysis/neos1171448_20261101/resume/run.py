"""neos-1171448 (Mittelmann) の trim3 の時間切れの切り分け: resume と d2lp1 を交互に解く (2026-11-01)。
4 スレッド・60 秒、ソルバーは同時に 1 つだけ。ENOMOTO_MIP_LOG=1。
使い方: python run.py 回数 [ログのディレクトリ (既定 logs)] [設定の並び (既定 resume,d2lp1)]
設定: def (既定、対照)・resume (ENOMOTO_MIP_RENS_RESUME=1)・d2lp1 (ENOMOTO_MIP_RENS_MAX_DEPTH=2 ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH=1)。
ログは <ログのディレクトリ>/<設定>/neos-1171448_r<回>.log (あれば飛ばす)。回ごとに resume → d2lp1 の順に交互に回す。"""
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
    "resume": {"ENOMOTO_MIP_RENS_RESUME": "1"},
    "d2lp1": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
}

if __name__ == "__main__":
    reps = int(sys.argv[1])
    L = D / (sys.argv[2] if len(sys.argv) > 2 else "logs")
    confs = sys.argv[3].split(",") if len(sys.argv) > 3 else ["resume", "d2lp1"]
    rb.fetch(P, C)
    for k in range(1, reps + 1):
        for c in confs:
            out = L / c / f"{P}_r{k}.log"
            if out.exists():
                continue
            out.parent.mkdir(parents=True, exist_ok=True)
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CONF[c])
            t0 = time.time()
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{P}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            out.write_text(f"# wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
            print(c, out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:200] if r.stdout.strip() else "NO OUTPUT", flush=True)
