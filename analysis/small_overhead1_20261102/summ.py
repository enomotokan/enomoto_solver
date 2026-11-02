"""results.jsonl を設定ごと・問題ごとに集計する (時間の中央値と各回、最適で終わらない回の数)。使い方: python summ.py [設定名 ...]"""
import json, sys, statistics as st
from collections import defaultdict
from pathlib import Path
R = defaultdict(list)
for l in (Path(__file__).parent / "results.jsonl").read_text().splitlines():
    r = json.loads(l); R[(r["cfg"], r["problem"])].append(r["result"])
cfgs = sys.argv[1:] or sorted({c for c, _ in R})
probs = sorted({p for _, p in R})
print(f"{'問題':12s}" + "".join(f"{c:>18s}" for c in cfgs))
for p in probs:
    if not any((c, p) in R for c in cfgs): continue
    row = f"{p:12s}"
    for c in cfgs:
        rs = R.get((c, p), [])
        if not rs: row += f"{'-':>18s}"; continue
        ts = [r.get("time", 60) if r.get("status", "").lower() == "optimal" else 60 for r in rs]
        bad = sum(r.get("status", "").lower() != "optimal" for r in rs)
        row += f"{st.median(ts):8.2f} ({len(rs)}{'!' + str(bad) if bad else ''}) " + f"{min(ts):4.1f}-{max(ts):<4.1f}"[:9]
    print(row)
