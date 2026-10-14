"""根の処理の時間の内訳と CPU 使用率を測る。

使い方: rootprof.py OUTDIR NAME [NAME ...]
各問題を threads=4 (ENOMOTO_MIP_THREADS=4) で 60 秒解き、詳細ログ (ENOMOTO_MIP_LOG=1) を OUTDIR/NAME.log に保存し、
プロセスの CPU 時間 (utime+stime、子スレッド込み) を 0.5 秒ごとに OUTDIR/NAME.cpu に記録する。
"""
import os
import subprocess
import sys
import time

SP = "/tmp/claude-0/-home-user-enomoto-solver/89e57c1f-baeb-54a0-97e3-7cc4061b50b5/scratchpad"
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
tick = os.sysconf("SC_CLK_TCK")
for name in sys.argv[2:]:
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_PROF_TREE_PAR="1")
    log = open(f"{out}/{name}.log", "w")
    t0 = time.time()
    p = subprocess.Popen([f"{SP}/venv/bin/python", f"{SP}/treepar/one2.py", f"/home/user/enomoto_solver/.miplib_cache/{name}.mps", "60", "4"],
                         stdout=log, stderr=subprocess.STDOUT, env=env, cwd="/tmp")
    samples = []
    while p.poll() is None:
        try:
            f = open(f"/proc/{p.pid}/stat").read().rsplit(")", 1)[1].split()
            cpu = (int(f[11]) + int(f[12])) / tick
            samples.append((time.time(), cpu))
        except Exception:
            pass
        time.sleep(0.5)
    with open(f"{out}/{name}.cpu", "w") as c:
        for t, v in samples:
            c.write(f"{t:.3f} {v:.2f}\n")
    print(name, "done", round(time.time() - t0, 1), flush=True)
