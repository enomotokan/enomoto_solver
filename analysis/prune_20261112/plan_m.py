"""PRUNE の 2 段目の分担表: Mittelmann で HiGHS に負ける問題 (analysis/hiloss_20261111.md) の 10 問 × 種 1・2 × {def, subfix0, prune3}、600 秒。
同じ (問題, 種) の 3 設定は同じ分担。時間の見積りは analysis/mitt600_20261103 の res9。使い方: python plan_m.py N → shards_M.json"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
PM = ["neos-873061", "unitcal_7", "pg5_34", "supportcase33", "tbfp-network", "fiball", "mzzv42z", "h80x6320d", "physiciansched6-2", "neos859080"]
CF = ["def", "subfix0", "prune3"]
N = int(sys.argv[1])
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/mitt600_20261103/res9_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); T[r["problem"]].append(min(r["result"].get("time") or 600, 600))
est = {p: (sum(T[p]) / len(T[p]) if T[p] else 600) + 20 for p in PM}
groups = sorted([(p, s) for p in PM for s in (1, 2)], key=lambda g: -est[g[0]])
load = [0.0] * N; shards = [[] for _ in range(N)]
for p, s in groups:
    i = min(range(N), key=lambda j: load[j]); shards[i] += [[p, c, s, 600] for c in CF]; load[i] += len(CF) * est[p]
(D / "shards_M.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print({p: round(est[p]) for p in PM}); print("見積り (時間):", [round(x / 3600, 2) for x in load])
