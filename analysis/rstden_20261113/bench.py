"""58 問 (analysis/cbcimpl_20261029/probs.py) × 種 × {c, c12p} 60 秒を run.py で 1 回ずつ (4 スレッド) 解き、bench.jsonl に足す
(analysis/rstden_20261113.md の 2 節)。同じ (問題, 種) の 2 設定は続けて、順は問題・種ごとに入れ替える。途中から再開できる。
使い方 (リポジトリの根で): python analysis/rstden_20261113/bench.py [種 ...]"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CF = ["c", "c12p"]
seeds = [int(a) for a in sys.argv[1:]] or [1, 2]
OUT = "bench.jsonl"
done = set()
if (D / OUT).exists():
    for l in (D / OUT).read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["cfg"], r["problem"], int(r["tl"]), r["rep"]))
for s in seeds:
    for i, p in enumerate(PROBS):
        cfs = CF if (i + s) % 2 == 0 else CF[::-1]
        for c in cfs:
            if (c, p, 60, s) not in done:
                subprocess.run([sys.executable, str(D / "run.py"), c, p, "60", str(s), OUT], check=False)
print("BENCH DONE", flush=True)
