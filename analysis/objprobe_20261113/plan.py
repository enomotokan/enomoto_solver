"""OBJPROBE の分担表を作る: python plan.py NP NH → shards_P.json・shards_H.json。
(P) 58 問 (analysis/cbcimpl_20261029/probs.py の PROBS) × 種 1〜3 × {base, c1, c2, c12} 60 秒。同じ (問題, 種) の 4 設定は同じ分担。
(H) HIGHSLOSS の 12 問 (analysis/highsloss_20261113/shards.json と同じ組) × 種 1 × {base, c12} 600 秒。同じ問題の 2 設定は同じ分担。
時間の見積りは cand1114 の P (analysis/cand1114_20261113/res_P_s*.jsonl の cand1114 の行) と HIGHSLOSS の cand の行、+ 起動の手間。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NP, NH = int(sys.argv[1]), int(sys.argv[2])
# 3 番目の引数が h なら h の確かめの組: Q (58 問 × 種 1〜3 × {base, c12, c12h} 60 秒) と G (12 問 × 種 1 × {base, c12, c12h} 600 秒)
HSET = len(sys.argv) > 3 and sys.argv[3] == "h"
CP, CH = (("base", "c12", "c12h"),) * 2 if HSET else (("base", "c1", "c2", "c12"), ("base", "c12"))
OP, OH = ("shards_Q.json", "shards_G.json") if HSET else ("shards_P.json", "shards_H.json")
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
TP = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1114_20261113/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114":
            TP[r["problem"]].append(min(r.get("wall") or 70.0, 75.0))
estP = lambda p: (sum(TP[p]) / len(TP[p]) if TP[p] else 70.0) + 2  # noqa: E731
gP = {(p, s): [(p, c, s) for c in CP] for p in PROBS for s in (1, 2, 3)}
balance(gP, estP, NP, OP, 60)
HL = sorted({p for s in json.loads((WT / "analysis/highsloss_20261113/shards.json").read_text())["shards"] for p, _ in s})
TH = defaultdict(list)
for f in glob.glob(str(WT / "analysis/highsloss_20261113/res_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r.get("cfg") == "cand":
            TH[r["problem"]].append(min(r.get("wall") or 650.0, 660.0))
estH = lambda p: (sum(TH[p]) / len(TH[p]) if TH[p] else 650.0) + 10  # noqa: E731
gH = {p: [(p, c, 1) for c in CH] for p in HL}
balance(gH, estH, NH, OH, 600)
