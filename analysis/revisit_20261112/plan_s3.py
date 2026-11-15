"""REVISIT の種 3 の分担表を作る: 58 問 × 種 3 × 種 1・2 で基準を満たした候補と def、60 秒。
候補 (2026-11-12、種 1・2 の 116 対の集計で区間の上端が +5% 以下・解けた回が減らない): lm2・prepar・sbt・ncm・ns250s・opf3。
使い方: python plan_s3.py N  → shards_Q.json (run.py Q K で回し、res_Q_sK.jsonl に書く)"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
C3 = ["def", "lm2", "prepar", "sbt", "ncm", "ns250s", "opf3"]
N = int(sys.argv[1])
T = defaultdict(list)
for f in glob.glob(str(D / "res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] in C3: T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = {p: (sum(T[p]) / len(T[p]) if T[p] else 40) + 4 for p in PROBS}
groups = sorted(PROBS, key=lambda p: -est[p])
load = [0.0] * N; shards = [[] for _ in range(N)]
for p in groups:
    i = min(range(N), key=lambda j: load[j])
    shards[i] += [[p, c, 3, 60] for c in C3]; load[i] += len(C3) * est[p]
(D / "shards_Q.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(PROBS), "問", sum(len(x) for x in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
