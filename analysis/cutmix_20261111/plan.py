"""分担表 shards.json を作る: 58 問 × 設定 def・cmh・cmfh × 種 1・2 (60 秒) のうち h58.jsonl にまだない回を、p58.jsonl (stat) の時間の
見積りで NS 個に釣り合うよう分ける。同じ (問題, 種) の 3 設定は同じ分担に入れる (組の比べを同じ機械で)。使い方: python plan.py NS"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NS = int(sys.argv[1])
CFGS = ["def", "cmh", "cmfh"]
T = defaultdict(list)
for l in open(D / "p58.jsonl"):
    r = json.loads(l)
    T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
done = {(r["problem"], r["cfg"], r["rep"]) for r in map(json.loads, open(D / "h58.jsonl")) }
groups = []
for s in (1, 2):
    for i, p in enumerate(PROBS):
        order = CFGS if i % 2 == 0 else CFGS[::-1]
        g = [[p, c, s] for c in order if (p, c, s) not in done]
        if g:
            est = (sum(T[p]) / len(T[p]) + 15) * len(g)
            groups.append((est, g))
groups.sort(key=lambda x: -x[0])
load = [0.0] * NS; shards = [[] for _ in range(NS)]
for est, g in groups:
    i = min(range(NS), key=lambda j: load[j]); shards[i] += g; load[i] += est
(D / "shards.json").write_text(json.dumps({"out": "h58", "tl": 60, "extra_cfg": "cmh=ENOMOTO_MIP_CUTMIX:h;cmfh=ENOMOTO_MIP_CUTMIX:fh",
                                           "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(sum(len(s) for s in shards), "回", "分担ごとの見積り (時間):", [round(x / 3600, 2) for x in load])
