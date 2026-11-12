"""PRUNE の分担表を作る: 58 問 (MIPLIB) × 種 1・2 × CFGS の全部、60 秒。
同じ (問題, 種) の全設定を同じ分担で続けて回す (機械・時期のずれを対で消す)。時間の見積りは cand1112 の P の def。
使い方: python plan.py N  → shards_P.json"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(D))
from probs import PROBS  # noqa: E402
from cfg import CFGS  # noqa: E402
N = int(sys.argv[1])
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1112_20261111/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = {p: (sum(T[p]) / len(T[p]) if T[p] else 40) + 4 for p in PROBS}
groups = sorted([(p, s) for p in PROBS for s in (1, 2)], key=lambda g: -est[g[0]])
load = [0.0] * N; shards = [[] for _ in range(N)]
for p, s in groups:
    i = min(range(N), key=lambda j: load[j])
    shards[i] += [[p, c, s, 60] for c in CFGS]; load[i] += len(CFGS) * est[p]
(D / "shards_P.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(PROBS), "問", sum(len(x) for x in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
