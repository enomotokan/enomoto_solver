"""neos-960392 を今の既定で何回も解いてログを取る (2026-11-01、「1. 解ける回と解けない回」)。
4 スレッド・60 秒、ソルバーは同時に 1 つだけ、ENOMOTO_MIP_LOG=1。
使い方: python run.py <接頭辞> <回数> [変数=値 ...]。ログは logs/<接頭辞>_<i>.log (既にあれば飛ばす)。
例: python run.py def 12 / python run.py vb 4 ENOMOTO_MIP_SUBMIP_VERBOSE=1"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
MPS = WT / ".miplib_cache" / "neos-960392.mps"
L = D / "logs"

if __name__ == "__main__":
    tag, n = sys.argv[1], int(sys.argv[2])
    extra = dict(a.split("=", 1) for a in sys.argv[3:])
    L.mkdir(exist_ok=True)
    for i in range(n):
        out = L / f"{tag}_{i}.log"
        if out.exists():
            continue
        env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **extra)
        t0 = time.time()
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(MPS),
                            "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
        out.write_text(f"# wall={time.time()-t0:.1f} env={extra}\n" + r.stderr + "\n" + r.stdout)
        print(out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:200] if r.stdout.strip() else "NO OUTPUT", flush=True)
