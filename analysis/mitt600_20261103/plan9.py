"""res9 (2026-11-11) の分担表 shards9.json を作る: Mittelmann 186 問 (shards2.json の問題) を、今の既定 (cand1108zd + DRCR + SYMDYN=f、
環境変数なし、解く側の名前 "ours") x 種 1・2 で、res7 (既定、種 1・2) の時間の見積りで NS 個に釣り合うよう分ける。使い方: python plan9.py NS"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
NS = int(sys.argv[1])
probs = sorted({p for s in json.loads((D / "shards2.json").read_text())["shards"] for p in s})
T = defaultdict(list)
for f in glob.glob(str(D / "res7s*_s*.jsonl")) + glob.glob(str(D / "res6s*_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["solver"] != "ours":
            continue
        x = r["result"]; st = (x.get("status") or "").lower(); t = x.get("time") or 600
        T[r["problem"]].append(t if st in ("optimal", "infeasible") and t <= 605 else 600)
est = {p: (sum(T[p]) / len(T[p]) if T[p] else 600) + 30 for p in probs}
tasks = [(p, "ours", seed) for p in probs for seed in (1, 2)]
tasks.sort(key=lambda t: -est[t[0]])
load_ = [0.0] * NS; shards = [[] for _ in range(NS)]
for t in tasks:
    i = min(range(NS), key=lambda j: load_[j]); shards[i].append(list(t)); load_[i] += est[t[0]]
(D / "shards9.json").write_text(json.dumps({"problems": probs, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load_]}, indent=0))
print(len(probs), "問", len(tasks), "回", "分担ごとの見積り (時間):", [round(x / 3600, 2) for x in load_])
