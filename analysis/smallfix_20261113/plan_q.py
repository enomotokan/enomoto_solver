"""SMALLFIX の d1g の 58 問の分担表: python plan_q.py NQ → shards_Q.json
(Q) 58 問 x 種 1〜3 x {base, d1g} 60 秒。同じ (問題, 種) の 2 設定は同じ分担。見積りは cand1114 の P (上限 60 秒) + 1 回 8 秒。
run.py Q K で回す (途中で止まっても res_Q_sK.jsonl の済んだ回を飛ばして続きから再開する)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
NQ = int(sys.argv[1])
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1114_20261113/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114":
            T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
assert len(T) == 58
groups = sorted(((2 * (max(T[p]) + 8), [[p, c, s, 60] for c in ("base", "d1g")]) for p in sorted(T) for s in (1, 2, 3)), key=lambda g: -g[0])
load = [0.0] * NQ; shards = [[] for _ in range(NQ)]
for est, runs in groups:
    i = min(range(NQ), key=lambda j: load[j]); shards[i] += runs; load[i] += est
(D / "shards_Q.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_Q.json", sum(len(s) for s in shards), "回、見積り (時間):", [round(x / 3600, 2) for x in load])
