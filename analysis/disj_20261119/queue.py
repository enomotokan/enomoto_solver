"""ジョブの一覧 (1 行に「問題 時間 種 出力 [root]」) を 1 本ずつ順に回す (4 スレッドで 1 本ずつ。機械は 4 コア)。
出力に同じ (問題, 種, root) の行があれば飛ばす (途中から再開できる)。使い方: python queue.py jobs.txt"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
for line in open(D / sys.argv[1]):
    a = line.split()
    if not a:
        continue
    p, tl, seed, out = a[0], a[1], a[2], a[3]
    root = len(a) > 4 and a[4] == "root"
    done = False
    if (D / out).exists():
        for l in open(D / out):
            r = json.loads(l)
            if r["problem"] == p and r["seed"] == int(seed) and r["root"] == root:
                done = True
    if not done:
        subprocess.run([sys.executable, str(D / "run.py"), *a], check=False)
