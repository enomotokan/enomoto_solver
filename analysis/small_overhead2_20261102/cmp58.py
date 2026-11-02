"""bench58.jsonl の 3 設定を比べる: 解けた数、SGM (シフト 10 秒、60 秒で解けない回は 60 秒)、既定との差が大きい問題。"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in (D / "bench58.jsonl").read_text().splitlines():
    r = json.loads(l); R[r["problem"]][r["cfg"]] = r["result"]
cfgs = ["def", "fjstop", "fjsl"]
ok = lambda r: r.get("status") == "optimal" and r.get("time", 99) <= 60
tt = lambda r: min(r["time"], 60.0) if ok(r) else 60.0
ps = [p for p in R if all(c in R[p] for c in cfgs)]
print(f"問題 {len(ps)}")
for c in cfgs:
    sgm = math.exp(sum(math.log(tt(R[p][c]) + 10) for p in ps) / len(ps)) - 10
    print(f"  {c:7s} 解けた {sum(ok(R[p][c]) for p in ps):2d}  SGM {sgm:6.2f}s")
print("既定との差が 0.5 秒以上 (かつ 10% 以上) の問題:")
for p in sorted(ps):
    d = tt(R[p]["def"])
    row = [tt(R[p][c]) for c in cfgs]
    if any(abs(x - d) >= 0.5 and abs(x - d) >= 0.1 * d for x in row[1:]):
        print(f"  {p:24s} " + "  ".join(f"{c}={tt(R[p][c]):5.1f}{'' if ok(R[p][c]) else '*'}" for c in cfgs))
