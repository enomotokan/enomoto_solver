"""手元 (16 スレッド) の計測の集計。jsonl を何個でも受け取り、設定 (cfg) ごとにまとめる。

使い方:
  python compare_local.py runs <jsonl ...> [-v]   回ごとの解けた数・SGM (10 秒シフト、未解決は 60 秒)・目的値の不一致、-v で問題ごとの時間
  python compare_local.py stats <jsonl ...>       問題 × 設定ごとの中央値・平均・最小〜最大・ノードの中央値

目的値の不一致は compare40.py と同じ (どれかの設定・回で最適と答えた値との比べ、相対 1e-4)。"""
import json, math, statistics, sys
from collections import defaultdict

mode, files = sys.argv[1], [a for a in sys.argv[2:] if not a.startswith("-")]
rows = [json.loads(l) for f in files for l in open(f, encoding="utf-8") if l.strip()]
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
t_of = lambda r: r["time"] if ok(r) else 60.0

if mode == "runs":
    print("| 設定 | 回 | 解けた数 | SGM [s] | 目的値の不一致 |")
    print("|---|---:|---:|---:|---|")
    for c in cfgs:
        for k in range(max(len(v) for v in runs[c].values())):
            rs = {p: runs[c][p][k] for p in probs if len(runs[c][p]) > k}
            bad = [p for p, r in rs.items() if ok(r) and abs(r["objective"] - ref[p]) > 1e-4 * max(1, abs(ref[p])) + 1e-6]
            print(f"| {c} | {k + 1} | {sum(ok(r) for r in rs.values())}/{len(rs)} | {sgm([t_of(r) for r in rs.values()]):.2f} | {len(bad)} {bad if bad else ''} |")
    if "-v" in sys.argv:
        print()
        print("| 問題 | " + " | ".join(cfgs) + " |")
        print("|---|" + "---:|" * len(cfgs))
        for p in probs:
            cell = lambda c: " / ".join(f"{r['time']:.1f}" if ok(r) else "t" for r in runs[c][p])
            print(f"| {p} | " + " | ".join(cell(c) for c in cfgs) + " |")
else:
    print("| 問題 | 設定 | 解けた回 | 中央値 [s] | 平均 [s] | 最小〜最大 [s] | ノード (中央値) |")
    print("|---|---|---:|---:|---:|---:|---:|")
    for p in probs:
        for i, c in enumerate(cfgs):
            rs = runs[c][p]
            ts = [t_of(r) for r in rs]
            nodes = statistics.median(r.get("nodes", 0) for r in rs)
            print(f"| {p if i == 0 else ''} | {c} | {sum(map(ok, rs))}/{len(rs)} | {statistics.median(ts):.2f} | {statistics.mean(ts):.2f} | "
                  f"{min(ts):.1f}〜{max(ts):.1f} | {nodes:g} |")
