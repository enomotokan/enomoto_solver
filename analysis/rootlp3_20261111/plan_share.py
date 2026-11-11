"""ROOTLP3 の分担表 (analysis/rootlp3_20261111.md §4)。58 問 x 種 1・2 の (問題, 種) ごとに既定 def と r3 を同じ分担で続けて回すよう、
p58.jsonl の既定の時間 (+ 10 秒) の見積りで N 個に釣り合わせ、shards.json に書く。使い方: python plan_share.py N"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1])
est = {}
for l in (D / "p58.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r["cfg"] == "def":
        est[(r["problem"], r["rep"])] = min(float(r["result"].get("time") or 60), 60) + 10
jobs = sorted(est.items(), key=lambda kv: -kv[1])
shards, load = [[] for _ in range(N)], [0.0] * N
for (p, s), t in jobs:
    k = load.index(min(load))
    shards[k] += [[p, "def", s], [p, "r3", s]] if len(shards[k]) % 4 == 0 else [[p, "r3", s], [p, "def", s]]
    load[k] += 2 * t
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_secs": [round(x) for x in load]}, ensure_ascii=False))
print([round(x / 60) for x in load], "分")
