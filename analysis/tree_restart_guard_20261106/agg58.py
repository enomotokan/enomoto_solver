"""S2/bench58.jsonl の集計 (2026-11-06): 回ごと・全回の SGM (10 秒ずらし、解けなかった回は 60 秒)、解けた数、片方だけ解けた問題、
既定より 1.5 倍かつ 3 秒以上遅い・速い問題。使い方: python agg58_s2.py [基準の設定 (既定 def)]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
base = sys.argv[1] if len(sys.argv) > 1 else "def"
R = defaultdict(dict)
for l in (D / "bench58.jsonl").read_text().splitlines():
    r = json.loads(l)
    res = r["result"]
    ok = str(res.get("status")).lower() == "optimal"
    R[(r["cfg"], r["rep"])][r["problem"]] = min(res.get("time") or 60.0, 60.0) if ok else 60.0
cfgs = sorted({c for c, _ in R}, key=lambda c: (c != base, c))
reps = sorted({k for _, k in R})
probs = sorted(set.intersection(*[set(v) for v in R.values()]))
print(f"問題 {len(probs)} (全設定・全回でそろったもの)")

def sgm(ts):
    return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10

print("| 設定 | " + " | ".join(f"種 {k}" for k in reps) + " | 全回 |")
print("|---|" + "---|" * (len(reps) + 1))
for c in cfgs:
    cells, allt, alls = [], [], 0
    for k in reps:
        ts = [R[(c, k)][p] for p in probs]
        s = sum(t < 60 for t in ts)
        cells.append(f"{sgm(ts):.2f} ({s})")
        allt += ts; alls += s
    b = sgm([R[(base, k)][p] for k in reps for p in probs])
    print(f"| {c} | " + " | ".join(cells) + f" | {sgm(allt):.2f} ({alls}) {100 * (sgm(allt) / b - 1):+.1f}% |")
for c in cfgs:
    if c == base:
        continue
    print(f"\n## {c} 対 {base}")
    for k in reps:
        a, b = R[(base, k)], R[(c, k)]
        only_b = [p for p in probs if a[p] >= 60 > b[p]]
        only_a = [p for p in probs if b[p] >= 60 > a[p]]
        slow = [f"{p} {a[p]:.1f}->{b[p]:.1f}" for p in probs if b[p] > 1.5 * a[p] and b[p] - a[p] > 3]
        fast = [f"{p} {a[p]:.1f}->{b[p]:.1f}" for p in probs if a[p] > 1.5 * b[p] and a[p] - b[p] > 3]
        print(f"- 種 {k}: {c} だけ解けた {only_b}、{base} だけ解けた {only_a}")
        print(f"  遅い {slow}")
        print(f"  速い {fast}")
