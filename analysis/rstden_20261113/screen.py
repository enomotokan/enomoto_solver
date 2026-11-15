"""絞り込みの計測 (analysis/rstden_20261113.md の 1 節): 問題 × 設定 × 種を順に run.py で 1 回ずつ (4 スレッド) 解き、screen.jsonl に足す。
途中から再開できる。使い方 (リポジトリの根で): python analysis/rstden_20261113/screen.py [種 ...]"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
P60 = ["h80x6320d", "physiciansched6-2", "10teams", "roll3000", "neos-911970", "p0201", "binkar10_1", "khb05250", "qnet1",
       "exp-1-500-5-5", "blp-ar98", "dcmulti", "neos17", "drayage-100-23", "mik-250-20-75-4", "swath1"]
P300 = ["mzzv42z", "neos-4722843-widden"]
CF = ["c", "c12p", "c2", "c12pf"]
seeds = [int(a) for a in sys.argv[1:]] or [1, 2]
OUT = "screen.jsonl"
done = set()
if (D / OUT).exists():
    for l in (D / OUT).read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["cfg"], r["problem"], int(r["tl"]), r["rep"]))
jobs = []
for s in seeds:
    for i, (p, tl) in enumerate([(p, 60) for p in P60] + [(p, 300) for p in P300]):
        # 設定の順を問題・種ごとに回す
        k = (i + s) % len(CF)
        cfs = CF[k:] + CF[:k]
        if p == "neos-4722843-widden":
            cfs = cfs + ["c1"]
        jobs += [(c, p, tl, s) for c in cfs]
for c, p, tl, s in jobs:
    if (c, p, tl, s) in done:
        continue
    subprocess.run([sys.executable, str(D / "run.py"), c, p, str(tl), str(s), OUT], check=False)
print("SCREEN DONE", flush=True)
