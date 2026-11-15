"""待ち行列 (続きから回せる): python queue.py 出力 時間 root|full 回のリスト 設定のリスト 問題のリスト (カンマ区切り、58 は probs.py の 58 問)。
出力に同じ (問題, 設定, 回, root) がある組は飛ばす。ソルバーは同時に 1 つ"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
out, tl, mode, reps, cfgs, probs = sys.argv[1:7]
if probs == "58":
    from probs import PROBS
    probs = PROBS
else:
    probs = probs.split(",")
root = mode == "root"
done = set()
if (D / out).exists():
    for l in open(D / out):
        r = json.loads(l)
        done.add((r["problem"], r["cfg"], r["rep"], r["root"]))
for rep in map(int, reps.split(",")):
    for p in probs:
        for c in cfgs.split(","):
            if (p, c, rep, root) in done:
                continue
            subprocess.run([sys.executable, str(D / "run.py"), c, p, tl, str(rep), out] + (["root"] if root else []))
