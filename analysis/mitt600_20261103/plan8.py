"""res8 の分担表 shards8.json を作る: HiGHS に負ける 47 問 (HiGHS が解き、res6 の既定が種 1・2 のどちらかで解けないか (t+10) の比 1/0.9 超) x
4 設定 x 種 1・2 を、res6 (既定)・res7 (cand1108z) の時間の見積りで NS 個に釣り合うよう分ける。使い方: python plan8.py NS"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
NS = int(sys.argv[1])
def load(pat):
    R = defaultdict(list)
    for f in glob.glob(str(D / pat)):
        for l in open(f):
            r = json.loads(l); x = r["result"]
            st = (x.get("status") or "").lower(); t = x.get("time") or 600
            ok = st in ("optimal", "infeasible") and t <= 605
            R[(r["problem"], r["solver"])].append((ok, t if ok else 600))
    return R
H = load("res_s*.jsonl"); E6 = load("res6s*_s*.jsonl"); E7 = load("res7s*_s*.jsonl")
probs = []
for p in sorted({p for p, s in H if s == "highs"}):
    h = H[(p, "highs")][0]; es = E6.get((p, "ours"), [])
    if h[0] and es and any((not e[0]) or (e[1] + 10) / (h[1] + 10) > 1 / 0.9 for e in es):
        probs.append(p)
est = {}
for p in probs:
    d = sum(e[1] for e in E6[(p, "ours")]) / 2
    z = E7.get((p, "ours:cand1108z")); z = sum(e[1] for e in z) / len(z) if z else d
    for s in ("ours", "ours:umr"): est[(p, s)] = d + 20
    for s in ("ours:cand1108z", "ours:cand1108zd"): est[(p, s)] = z + 20
tasks = [(p, s, seed) for p in probs for s in ("ours", "ours:umr", "ours:cand1108z", "ours:cand1108zd") for seed in (1, 2)]
tasks.sort(key=lambda t: -est[t[:2]])
load_ = [0.0] * NS; shards = [[] for _ in range(NS)]
for t in tasks:
    i = min(range(NS), key=lambda j: load_[j]); shards[i].append(list(t)); load_[i] += est[t[:2]]
(D / "shards8.json").write_text(json.dumps({"problems": probs, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load_]}, indent=0))
print(len(probs), "問", len(tasks), "回", "分担ごとの見積り (時間):", [round(x / 3600, 2) for x in load_])
