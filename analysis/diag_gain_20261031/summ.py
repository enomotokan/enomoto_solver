"""res.jsonl の集計: 問題 × 設定ごとに、各回の時間 (時間切れは T、60 秒超は時間切れとして数える) と解けた数を出す。
使い方: python summ.py [設定,...] [問題,...]"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(list)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    R[(r["problem"], r["config"])].append(r)
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else sorted({c for _, c in R})
probs = sys.argv[2].split(",") if len(sys.argv) > 2 else sorted({p for p, _ in R})
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    cells = []
    for c in cfgs:
        rs = sorted(R.get((p, c), []), key=lambda r: r["rep"])
        ok = [r["status"] == "optimal" and r["time"] <= 60 for r in rs]
        ts = ["%.0f" % r["time"] if o else "T" for r, o in zip(rs, ok)]
        cells.append(f"{'・'.join(ts)} ({sum(ok)}/{len(rs)})" if rs else "")
    print(f"| {p} | " + " | ".join(cells) + " |")
