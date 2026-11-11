"""(2) 畳んだ問題の上の GAP1 の根のプロービング `u` の分担表 shards_gap1u.json を作る (analysis/drcr2_20261111.md)。
7 問 x 設定 def・fu・fui・nfu (drcr_20261110/run.py の CFGS) x 種 1・2 を、300 秒、時間の見積りで NS 個に釣り合うよう分ける。
見積り: 既定で解ける 3 問は今までの計測の時間 (order32.jsonl 等) を多めに、残りは時間切れの 300 秒 + 30 秒。使い方: python plan_gap1u.py NS"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
NS = int(sys.argv[1])
EST = {"neos-957323": 60, "neos-960392": 60, "physiciansched6-2": 120, "fiball": 330, "sct2": 330, "seymour1": 330,
       "neos-4722843-widden": 330}
tasks = [(p, c, s) for p in EST for c in ("def", "fu", "fui", "nfu") for s in (1, 2)]
tasks.sort(key=lambda t: -EST[t[0]])
load_ = [0.0] * NS; shards = [[] for _ in range(NS)]
for t in tasks:
    i = min(range(NS), key=lambda j: load_[j]); shards[i].append(list(t)); load_[i] += EST[t[0]]
(D / "shards_gap1u.json").write_text(json.dumps({"tl": 300, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load_]}, indent=0))
print(len(tasks), "回", "分担ごとの見積り (時間):", [round(x / 3600, 2) for x in load_])
