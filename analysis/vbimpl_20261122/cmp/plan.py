"""VB_IMPL の比べの分担表: VB_IMPL が変数上下限を足す 8 問 (count.jsonl) × 種 1〜3 × {def, vb (=1), vbt (=t)}、4 スレッド。
P の 2 問 (timtab1・h80x6320d) は 60 秒、ほかは 600 秒。python plan.py 6 → shards.json"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
NS = int(sys.argv[1])
PROBS = {"unitcal_7": (600, 350), "h80x6320d": (60, 35), "rocI-4-11": (600, 300), "neos-1456979": (600, 300), "neos-3627168-kasai": (600, 300),
         "fastxgemm-n2r6s0t2": (600, 60), "timtab1": (60, 25), "tr12-30": (600, 150)}
CF = ["def", "vb", "vbt"]
groups = []
for p, (tl, est) in PROBS.items():
    for s in (1, 2, 3):
        cf = CF[s - 1:] + CF[:s - 1]  # 設定の順を種で回す
        groups.append(([[p, c, s, tl] for c in cf], est * 3))
groups.sort(key=lambda g: -g[1])
load = [0.0] * NS; shards = [[] for _ in range(NS)]
for ts, est in groups:
    i = min(range(NS), key=lambda j: load[j]); shards[i] += ts; load[i] += est
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print([round(x / 3600, 2) for x in load])
