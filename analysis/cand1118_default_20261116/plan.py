"""分担表: python plan.py N → shards_P.json。P 58 問 × 種 1 × {def_t1, c1118_t1 (1 スレッド、節点数まで一致するかの比べ), def, rev (4 スレッド)}、60 秒 (232 回)。
同じ問題の 3 設定は同じ分担で続けて解く。見積りは cand1118 の回の cand1118 の時間 (+ 立ち上げ)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
T = defaultdict(list)
for f in glob.glob(str(D.parent / "cand1118_20261116/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1118": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = {p: (sum(T[p]) / len(T[p]) if T[p] else 48) + 8 for p in PROBS}
N = int(sys.argv[1]); load = [0.0] * N; shards = [[] for _ in range(N)]
for p in sorted(PROBS, key=lambda q: -est[q]):
    i = min(range(N), key=lambda j: load[j])
    shards[i] += [[p, c, 1, 60] for c in ("def_t1", "c1118_t1", "def", "rev")]; load[i] += 2 * min(60 + 8, 2.5 * est[p]) + 2 * est[p]
(D / "shards_P.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_P.json:", N, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
