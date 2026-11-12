"""PRESOLVE2 の分担表を作る (TREEEFF の plan.py の P と同じ作り)。
P (58 問 × 種 1〜3、60 秒): 既定と候補を同じ分担で対にして回す。使い方: python plan.py P N 候補 [候補 ...] → shards_P.json"""
import json, sys, glob
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
T2 = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1111_20261111/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r.get("cfg") == "def" and isinstance(r["result"].get("time"), (int, float)): T2[r["problem"]].append(min(r["result"]["time"], 60))
E = {p: (sum(T2[p]) / len(T2[p]) if T2[p] else 40) + 6 for p in PROBS}
n = int(sys.argv[2]); cands = sys.argv[3:]
tasks = [(p, c, s) for p in PROBS for s in (1, 2, 3) for c in ["def"] + cands]
groups = defaultdict(list)
for t in tasks: groups[(t[0], t[2])].append(t)
gl = sorted(groups.items(), key=lambda kv: -len(kv[1]) * E[kv[0][0]])
load = [0.0] * n; shards = [[] for _ in range(n)]
for _, ts in gl:
    i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [60] for t in ts]; load[i] += len(ts) * E[ts[0][0]]
(D / "shards_P.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(tasks), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
