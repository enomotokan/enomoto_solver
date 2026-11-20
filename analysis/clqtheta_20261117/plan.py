"""分担表: python plan.py N → shards.json。同じ (問題, 種) の 8 設定 (def と θ 7 つ) は同じ分担 (= 同じ機械) で続けて解く。
見積りは clqlazy_20261117 の def・lazy の時間の平均 (なければ上限) × 8 + 起動の分。"""
import json, glob, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS, P, M, SEEDS, TL  # noqa: E402
T = collections.defaultdict(list)
for f in glob.glob(str(D.parent / "clqlazy_20261117/res_*_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        st = (r["result"].get("status") or "").lower()
        T[r["problem"]].append(min(r["result"].get("time") or r["tl"], r["tl"]) if st in ("optimal", "infeasible") else r["tl"])
def est(p, tl): return (min(sum(T[p]) / len(T[p]), tl) if T[p] else tl) + 8
groups = [[[p, c, s, TL[S]] for c in CFGS] for S, probs in (("P", P), ("M", M)) for p in probs for s in SEEDS]
n = int(sys.argv[1]); load = [0.0] * n; shards = [[] for _ in range(n)]
for g in sorted(groups, key=lambda g: -est(g[0][0], g[0][3])):
    i = min(range(n), key=lambda j: load[j]); shards[i] += g; load[i] += len(g) * est(g[0][0], g[0][3])
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards.json", n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
