"""群 5 can の追加の分担表: python plan_extra.py N → shards_MCAN2.json (N 分担)。
MCAN (種 1・2) で種ごとに向きが逆・片方だけ解けた 8 問を、種 3・4 × {def, can}・600 秒で足す。
見積りは MCAN の同じ問題の 2 設定・2 種の時間の平均 (時間切れは 600 秒) + 1 回 30 秒。同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。
回し方は run.py と同じ: python run.py MCAN2 K。集計は MCAN と合わせて python summ.py MCAN can (res_MCAN2_s*.jsonl も拾うように summ.py の SET を MCAN2 にして別に見る)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1])
PROBS = ["fiball", "neos-933966", "neos-662469", "peg-solitaire-a3", "tbfp-network", "traininstance6", "mzzv11", "neos-1456979"]
T = defaultdict(list)
for f in glob.glob(str(D / "res_MCAN_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        T[r["problem"]].append(min(r["result"].get("time") or 600, 600))
est = {p: sum(T[p]) / len(T[p]) + 30 for p in PROBS}
groups = {(p, s): [(p, c, s) for c in (["def", "can"] if s % 2 else ["can", "def"])] for p in PROBS for s in (3, 4)}
gl = sorted(groups.items(), key=lambda kv: -2 * est[kv[0][0]])
load = [0.0] * N; shards = [[] for _ in range(N)]
for (p, s), ts in gl:
    i = min(range(N), key=lambda j: load[j]); shards[i] += [list(t) + [600] for t in ts]; load[i] += 2 * est[p]
(D / "shards_MCAN2.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_MCAN2.json", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
