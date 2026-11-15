"""bgdse_grow の当たりつけ: 写しの根の LP の後の DSE 重みの育ち具合 (α = 0 で計算し直さず表示だけ) を見て、表示が出たら止める。
使い方: python probe.py 問題,... [設定名]   ログ probe/<問題>_<設定名>.log。LP_WORK=1 + DSE=g + GROW=0、4 スレッド・60 秒"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
tag = sys.argv[2] if len(sys.argv) > 2 else "g0"
for P in sys.argv[1].split(","):
    mps = rb.fetch(P, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", ENOMOTO_MIP_BG_CUTS_LP_WORK="1",
               ENOMOTO_MIP_BG_CUTS_DSE="g", ENOMOTO_T_MIP_BG_CUTS_DSE_GROW="0")
    log = D / "probe" / f"{P}_{tag}.log"
    with open(log, "w") as f:
        p = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                             cwd=WT, env=env, stdout=f, stderr=subprocess.STDOUT)
        t0 = time.time()
        while p.poll() is None and time.time() - t0 < 90:
            time.sleep(0.5)
            if "after the root LP" in log.read_text(errors="replace"):
                time.sleep(1)
                break
        p.kill(); p.wait()
    l = [x for x in log.read_text(errors="replace").splitlines() if "after the root LP" in x or "column values" in x]
    print(P, *l, sep="\n  ", flush=True)
