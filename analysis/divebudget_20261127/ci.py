"""採用基準の照合 (analysis/lpwarm_20261118/ci58.py の写し、回の名前は seed): ずらし幾何平均の比 (ずらし 10 秒、解けない回は TL) の 95% 区間、解けた数、
全回で 1.25 倍超の遅れ (素の時間の比と、ずらした比) の問題。使い方: python ci58.py TL 既定 試作 jsonl..."""
import json, math, sys
from collections import defaultdict
T = float(sys.argv[1]); base, proto = sys.argv[2], sys.argv[3]
by = defaultdict(dict)
for f in sys.argv[4:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l); by[r["problem"]][(r["cfg"], r["seed"])] = r["result"]
t = lambda res: min(res.get("time") or T, T) if res.get("status") == "optimal" else T
logs, slow, solved = [], [], [0, 0]
for p, d in by.items():
    reps = sorted({k[1] for k in d if (base, k[1]) in d and (proto, k[1]) in d})
    if not reps:
        continue
    lr = [math.log((t(d[(proto, r)]) + 10) / (t(d[(base, r)]) + 10)) for r in reps]
    logs.append(sum(lr) / len(lr))
    solved[0] += sum(d[(base, r)].get("status") == "optimal" for r in reps); solved[1] += sum(d[(proto, r)].get("status") == "optimal" for r in reps)
    raw = [t(d[(proto, r)]) / max(t(d[(base, r)]), 1e-9) for r in reps]
    sh = [math.exp(x) for x in lr]
    if all(x > 1.25 for x in raw) or all(x > 1.25 for x in sh):
        slow.append((p, [round(x, 2) for x in raw], [round(x, 2) for x in sh]))
n = len(logs); m = sum(logs) / n; sd = (sum((x - m) ** 2 for x in logs) / (n - 1)) ** 0.5; h = 1.96 * sd / n ** 0.5
print(f"問題 {n}: ずらし幾何平均の比 {math.exp(m):.3f} (95% [{math.exp(m - h):.3f}, {math.exp(m + h):.3f}])、解けた {base} {solved[0]} / {proto} {solved[1]}")
print("全回で 1.25 倍超の遅れ (素の比, ずらした比):", slow if slow else "なし")
