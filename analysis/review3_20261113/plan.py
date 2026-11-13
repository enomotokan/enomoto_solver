"""REVIEW3 の分担表 shards_P.json を作る: 58 問 × 種 1〜4 × 5 設定 (base・rmc100・rmc150・rmc300・mush0) × 60 秒。
同じ (問題, 種) の 5 設定は同じ分担で続けて回す (設定の順は (問題, 種) ごとに回して並べる)。見積りは cand1113 の P の回の時間 + 8 秒。
python plan.py N"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
N = int(sys.argv[1]); TL = 60
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1113_20261112/res_P_s*.jsonl")):
    for l in open(f):
        try: r = json.loads(l); t = r["result"].get("time")
        except Exception: continue
        if isinstance(t, (int, float)): T[r["problem"]].append(min(t, TL))
est = {p: (sum(T[p]) / len(T[p]) if T[p] else 40) + 8 for p in PROBS}
cfgs = list(CFGS)
groups = sorted(((p, s) for p in PROBS for s in (1, 2, 3, 4)), key=lambda g: -est[g[0]])
load = [0.0] * N; shards = [[] for _ in range(N)]
for gi, (p, s) in enumerate(groups):
    k = min(range(N), key=lambda i: load[i]); rot = gi % len(cfgs)
    shards[k] += [[p, c, s, TL] for c in cfgs[rot:] + cfgs[:rot]]
    load[k] += len(cfgs) * est[p]
(D / "shards_P.json").write_text(json.dumps({"est_hours": [round(x / 3600, 2) for x in load], "shards": shards}, indent=0))
print(sum(len(s) for s in shards), "回、見積り (時間):", [round(x / 3600, 2) for x in load])
