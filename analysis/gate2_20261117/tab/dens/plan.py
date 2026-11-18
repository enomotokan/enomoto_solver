"""密度で割る点数の試作の分担表: P = MIPLIB の 58 問 × 種 1・2 × {def, b1}、60 秒、1 スレッド。1 分担 ≤ 1.5 時間
(見積りは tab/res_P (4 スレッド) の def の時間 × 2 (1 スレッド) + 8 秒、上限 68 秒)。python plan.py → shards_P.json"""
import json, glob
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
P = sorted({x[0] for sh in json.loads((D.parent / "shards_P.json").read_text())["shards"] for x in sh})
T = defaultdict(list)
for f in glob.glob(str(D.parent / "res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = lambda p: min(2 * sum(T[p]) / len(T[p]) if T[p] else 60, 60) + 8
groups = [((p, s), [(p, c, s, 60) for c in (["def", "b1"] if s % 2 else ["b1", "def"])]) for p in P for s in (1, 2)]
groups.sort(key=lambda g: -est(g[0][0]))
n = 1
while True:
    load = [0.0] * n; sh = [[] for _ in range(n)]
    for (p, s), ts in groups:
        i = min(range(n), key=lambda j: load[j]); sh[i] += [list(t) for t in ts]; load[i] += 2 * est(p)
    if max(load) <= 1.5 * 3600: break
    n += 1
(D / "shards_P.json").write_text(json.dumps({"shards": sh, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_P.json", n, "分担", sum(len(x) for x in sh), "回", [round(x / 3600, 2) for x in load])
