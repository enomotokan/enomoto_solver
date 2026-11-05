"""bench58.jsonl を集計する: 設定ごとの解けた数・SGM (10 秒ずらし、解けない・60 秒超えは 60 秒)、試作 (HO4) が働いた問題、1.3 倍以上違った問題。
使い方: python agg58.py [基準の設定 (def)] [比べる設定,...]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
W = defaultdict(dict)
for l in open(D / "bench58.jsonl"):
    r = json.loads(l); res = r["result"]
    ok = res.get("status") == "optimal" or res.get("status") == "infeasible"
    t = min(res.get("time", 60.0), 60.0) if ok and res.get("time", 99) <= 60.0 else 60.0
    R[r["cfg"]].setdefault(r["problem"], []).append(t)
    W[r["cfg"]][r["problem"]] = ((res.get("ho4", 0),), res.get("restarts"), res.get("time"))
base = sys.argv[1] if len(sys.argv) > 1 else "def"
others = sys.argv[2].split(",") if len(sys.argv) > 2 else [c for c in R if c != base]
def mean(v): return sum(v) / len(v)
def sgm(c, ps): return math.exp(sum(math.log(mean(R[c][p]) + 10) for p in ps) / len(ps)) - 10
for c in [base] + others:
    ps = [p for p in R[base] if p in R[c]]
    print(f"{c}: {len(ps)} 問、解けた {sum(1 for p in ps if mean(R[c][p]) < 60)}、SGM(+10) {sgm(c, ps):.2f} 秒 (基準 {sgm(base, ps):.2f})")
for c in others:
    ps = [p for p in R[base] if p in R[c]]
    fired = [p for p in ps if any(W[c][p][0])]
    print(f"\n## {c}: 試作が働いた問題 {len(fired)} 問 (SGM 基準 {sgm(base, fired):.2f} → {sgm(c, fired):.2f})" if fired else f"\n## {c}: 働いた問題なし")
    print("| 問題 | 基準 | 試作 | 比 | 働いた試作 (HO4 の行数) | 再スタート (基準 → 試作) | 試作の時間 (60 秒超えを見る) |")
    print("|---|---|---|---|---|---|---|")
    for p in ps:
        a, b = mean(R[base][p]), mean(R[c][p])
        big = (b + 10) / (a + 10)
        if p in fired or big > 1.3 or big < 1 / 1.3:
            print(f"| {p} | {a:.2f} | {b:.2f} | {big:.2f} | {'/'.join(map(str, W[c][p][0]))} | {W[base][p][1]} → {W[c][p][1]} | {W[c][p][2]:.1f} |")
