"""問題 × 設定の時間 (秒、時間切れは 60) の平均と、全問の幾何平均 (シフト 10)。使い方: python table.py [ログのディレクトリ]"""
import json, math, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / (sys.argv[1] if len(sys.argv) > 1 else "logs")
CF = ["real_s0", "real_s2", "def_s0", "def", "real"]
T = defaultdict(list)
for f in sorted(L.glob("*.log")):
    for c in CF:
        m = re.match(rf"(.+)_{c}_(\d+)\.log$", f.name)
        if m:
            break
    else:
        continue
    res = [json.loads(l) for l in f.read_text(errors="replace").splitlines() if l.startswith("{") and '"status"' in l]
    if res:
        r = res[-1]
        T[(m[1], c)].append(min(r["time"], 60.0) if r["status"] == "optimal" else 60.0)
probs = sorted({k[0] for k in T})
cfgs = [c for c in ["def", "real", "real_s0", "real_s2"] if any(k[1] == c for k in T)]
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    print(f"| {p} | " + " | ".join(" / ".join(f"{t:.1f}" for t in T[(p, c)]) + (f" (平均 {sum(T[(p,c)])/len(T[(p,c)]):.1f})" if T[(p, c)] else "") for c in cfgs) + " |")
for c in cfgs:
    ps = [p for p in probs if all(T[(p, cc)] for cc in cfgs)]
    g = math.exp(sum(math.log(sum(T[(p, c)]) / len(T[(p, c)]) + 10) for p in ps) / len(ps)) - 10
    print(f"{c}: 共通 {len(ps)} 問の平均の SGM(10) {g:.2f}")
