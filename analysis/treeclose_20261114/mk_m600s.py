"""600 秒の 15 問 × 種 2〜3 の対 (base2・tca、4 スレッド) の分担表 m600s.json を作る: python mk_m600s.py [分担の数 (既定 6)]。
同じ (問題, 種) の 2 設定は同じ分担に入れ、続けて回す (同じ機械・同時期の対)。種 1 は m600 にある。
tcaf (TC=af) は 1 スレッドの 5 問で tca を上回らなかったので外した (記録の 6 章)。
回し方: python run.py K m600s.json  (K = 0..分担の数-1、res_m600s_sK.jsonl とログ logs/<問題>_<設定>_s<種>.log.gz)。"""
import json, sys
from pathlib import Path

D = Path(__file__).resolve().parent
m = json.loads((D / "m600.json").read_text())
probs = []
for sh in m["shards"]:
    for it in sh:
        if it[0] not in probs:
            probs.append(it[0])
cfgs = ["base2", "tca"]
groups = [[(p, c, s) for c in cfgs] for s in (2, 3) for p in probs]
# m600 の種 1 の時間 (時間切れは 600) を問題の重さに使い、重い組から軽い分担へ入れる
w = {}
for f in sorted(D.glob("res_m600_s*.jsonl")):
    for l in f.read_text().splitlines():
        if l.strip():
            r = json.loads(l)
            w[r["problem"]] = max(w.get(r["problem"], 0), min(600, r["result"].get("time", 600)))
k = int(sys.argv[1]) if len(sys.argv) > 1 else 4
shards, load = [[] for _ in range(k)], [0.0] * k
for g in sorted(groups, key=lambda g: -w.get(g[0][0], 600) * len(g)):
    i = load.index(min(load))
    shards[i] += [list(x) for x in g]
    load[i] += w.get(g[0][0], 600) * len(g)
plan = {"tl": 600, "seed": 1, "cfgs": {"base2": {}, "tca": {"ENOMOTO_MIP_TC": "a"}}, "shards": shards}
(D / "m600s.json").write_text(json.dumps(plan, indent=1))
print(len(probs), "問", sum(len(s) for s in shards), "回", "分担ごとの見込み (時間):", [round(x / 3600, 1) for x in load])
