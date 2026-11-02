"""res.jsonl から、問題 × 設定ごとの回の結果と SGM (シフト 1 秒、時間切れは 60 秒) を出す (sgm.txt)。"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
d = defaultdict(list)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if r["config"] in ("def", "mr2", "bx", "bxl"):
        d[(r["problem"], r["config"])].append(r["time"] if r["status"] == "optimal" else None)
out = []
for (p, c), v in sorted(d.items()):
    t = [min(x, 60.0) if x is not None else 60.0 for x in v]
    sgm = math.exp(sum(math.log(x + 1) for x in t) / len(t)) - 1
    s = "・".join(f"{x:.1f}" if x is not None else "T" for x in v)
    out.append(f"{p:18s} {c:4s} {sum(x is not None for x in v)}/{len(v)} SGM {sgm:5.1f}  {s}")
(D / "sgm.txt").write_text("\n".join(out) + "\n")
print("\n".join(out))
