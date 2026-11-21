"""cond の判断だけを見る (decide.py の速い版): ENOMOTO_MIP_ORBPP=cond・1 スレッドで回し、対称性の段の行 (「ORBPP cond:」・「symmetry: none」・
「N generators,」) が出たら止める。python decide2.py 問題... >> decide_M2.txt。行は「問題 判断 | ORBPP の行 | オービトープの行」"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_ORBPP": "cond", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1"})
for p in sys.argv[1:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "3000", "--threads", "1"],
                          cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    cond, orbpp, orbs, end = "", "", "", "時間切れ (1800 秒)"
    t0 = time.time()
    for line in pr.stderr:
        if "ORBPP cond:" in line: cond = line.strip()
        elif "ORBPP " in line and "orbitopes static" in line: orbpp = line.strip()
        elif "full orbitopes" in line: orbs = line.strip()
        if "symmetry: none" in line: end = "対称性なし"; break
        if "generators," in line: end = "対称性あり"; break
        if "solving the root LP" in line or time.time() - t0 > 1800: end = "根の LP まで行った" if "root" in line else end; break
    pr.kill(); pr.wait()
    dec = ("静的" if "-> static" in cond else "動的に戻す") if cond else "判断なし (既定と同じ問題)"
    print(p, dec, "|", end, "|", cond.replace("MIP: symmetry: ", ""), "|", orbpp.replace("MIP: symmetry: ", ""), "|", orbs.replace("MIP: symmetry: ", "")[:200], flush=True)
