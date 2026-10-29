"""miplib40.jsonl の設定ごとの集計 (各回の解けた数・SGM (10 秒シフト、未解決は 60 秒)・目的値の不一致) と問題ごとの時間。

目的値は、どちらかの設定・回で最適と答えた値を参照値にして、最適と答えた回のうち合わないものを不一致とする。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path

here = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(here / "miplib40.jsonl")]
runs = defaultdict(lambda: defaultdict(list))  # cfg -> problem -> [result]
for r in rows:
    runs[r["cfg"]][r["problem"]].append(r["result"])
cfgs = list(dict.fromkeys(r["cfg"] for r in rows))
probs = list(dict.fromkeys(r["problem"] for r in rows))
ok = lambda r: r.get("status") in ("optimal", "Optimal")
ref = {}
for p in probs:
    vals = [r["objective"] for c in cfgs for r in runs[c][p] if ok(r)]
    if vals:
        ref[p] = vals[0]
sgm = lambda ts: math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
print("| 設定 | 回 | 解けた数 | SGM [s] | 目的値の不一致 |")
print("|---|---:|---:|---:|---|")
for c in cfgs:
    for k in range(max(len(v) for v in runs[c].values())):
        rs = {p: runs[c][p][k] for p in probs if len(runs[c][p]) > k}
        ts = [r["time"] if ok(r) else 60.0 for r in rs.values()]
        bad = [p for p, r in rs.items() if ok(r) and abs(r["objective"] - ref[p]) > 1e-4 * max(1, abs(ref[p])) + 1e-6]
        print(f"| {c} | {k + 1} | {sum(ok(r) for r in rs.values())}/{len(rs)} | {sgm(ts):.2f} | {len(bad)} {bad if bad else ''} |")
if "-v" in sys.argv:
    print()
    print("| 問題 | " + " | ".join(cfgs) + " |")
    print("|---|" + "---:|" * len(cfgs))
    for p in probs:
        cell = lambda c: " / ".join(f"{r['time']:.1f}" if ok(r) else "t" for r in runs[c][p])
        print(f"| {p} | " + " | ".join(cell(c) for c in cfgs) + " |")
