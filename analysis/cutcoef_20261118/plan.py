"""cutcoef 段 1 の分担表: python plan.py N → shards_G.json
G: 代表 18 問 (P の中で既定が 60 秒以内に解け、1 秒を超えるもの。根が退化 roll3000・mc11、固定費用型 sp150x300d・p200x1188c、
密なカット qiu・rout など) × 種 1〜3 × cfg.py の 11 組、4 スレッド・60 秒。同じ (問題, 種) の 11 組は同じ分担で続けて解き、順は種でずらす。
見積り: vshighs_20261117 の def の時間 + 8 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
N = int(sys.argv[1])
PROBS = ["sp150x300d", "p200x1188c", "misc07", "rout", "10teams", "qiu", "mas76", "neos-911970", "pk1", "eil33-2", "swath1", "timtab1",
         "mc11", "roll3000", "h80x6320d", "hypothyroid-k1", "noswot", "neos5"]
T = defaultdict(list)
for f in glob.glob(str(D.parent / "vshighs_20261117/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = {p: sum(T[p]) / len(T[p]) + 8 if T[p] else 60 for p in PROBS}
cs = list(CFGS)
groups = {(p, s): [(p, cs[(i + s) % len(cs)], s) for i in range(len(cs))] for p in PROBS for s in (1, 2, 3)}
gl = sorted(groups.items(), key=lambda kv: -est[kv[0][0]])
load = [0.0] * N; shards = [[] for _ in range(N)]
for (p, s), ts in gl:
    i = min(range(N), key=lambda j: load[j]); shards[i] += [list(t) + [60] for t in ts]; load[i] += est[p] * len(ts)
(D / "shards_G.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
