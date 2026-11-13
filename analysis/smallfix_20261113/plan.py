"""SMALLFIX の分担表を作る: python plan.py NP NH → shards_P.json・shards_H.json
(P) 58 問 x 種 1〜3 x {base, c3, d1, c3d1} 60 秒。同じ (問題, 種) の 4 設定は同じ分担に入れる (時期・機械のずれを対で消す)。
(H) HIGHSLOSS の 12 問 (analysis/highsloss_20261113/shards.json) x 種 1 x {base, c3d1} 600 秒。同じ問題の 2 設定は同じ分担。
時間の見積りは cand1114 の P (res_P、cand1114 の列、上限 60 秒) と、H は 600 秒 (解けた問題も上限で数える) + 1 回 15 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
NP, NH = int(sys.argv[1]), int(sys.argv[2])
CF = ["base", "c3", "d1", "c3d1"]
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1114_20261113/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114":
            T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
probs = sorted(T)
assert len(probs) == 58, len(probs)
def balance(groups, n, out):
    gl = sorted(groups, key=lambda g: -g[0])
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for est, runs in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += est
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
gp = []
for p in probs:
    est1 = max(T[p]) + 8
    for seed in (1, 2, 3):
        gp.append((4 * est1, [[p, c, seed, 60] for c in CF]))
balance(gp, NP, "shards_P.json")
H = sorted({p for s in json.loads((WT / "analysis/highsloss_20261113/shards.json").read_text())["shards"] for p, _ in s})
balance([(2 * 615, [[p, c, 1, 600] for c in ("base", "c3d1")]) for p in H], NH, "shards_H.json")
