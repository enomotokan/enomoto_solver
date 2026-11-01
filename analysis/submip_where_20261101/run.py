"""サブ MIP がどこで解を見つけるかの調べ (2026-11-01)。今の既定で 58 問を 1 回ずつ (4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1 (サブ MIP の中の経過と、入れ子を区別する `SMT ...` の行も出す)。
使い方: python run.py [問題,...] [ログのディレクトリ 変数=値 ...]。ログは logs/<問題>.log (既定)。
カットの効きの比べ (記録の 5 節): python run.py <18 問> logs_nocut ENOMOTO_MIP_SUBMIP_HIGHS=-sc"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
L = D / "logs"
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS

if __name__ == "__main__":
    probs = sys.argv[1].split(",") if len(sys.argv) > 1 else PROBS
    if len(sys.argv) > 2:
        L = D / sys.argv[2]
    extra = dict(a.split("=", 1) for a in sys.argv[3:])
    L.mkdir(exist_ok=True)
    for p in probs:
        out = L / f"{p}.log"
        if out.exists() or (L / f"{p}.log.gz").exists():
            continue
        env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SUBMIP_VERBOSE="1", ENOMOTO_MIP_THREADS="4", **extra)
        t0 = time.time()
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                            "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
        out.write_text(f"# wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
        print(out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
