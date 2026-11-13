"""分担表を作る: python plan.py ND NP → shards_D.json・shards_P.json。同じ (問題, 種) の cand1114 と pint は同じ分担に入れ、
交互に回す。D: 重複の整数列のある 8 問 × 種 1〜4、P: 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1・2 (D と重なる問題も入れる)。
時間の見積りは cand1114 の P の回 (analysis/cand1114_20261113/res_P_s*.jsonl)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
ND, NP = int(sys.argv[1]), int(sys.argv[2])
DUP = ["nw04", "physiciansched6-2", "mzzv42z", "air03", "mas74", "mas76", "swath1", "neos-960392"]
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1114_20261113/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = defaultdict(lambda: 48.0, {p: sum(v) / len(v) + 8 for p, v in T.items()})
def balance(groups, n, out):
    gl = sorted(groups.items(), key=lambda kv: -2 * est[kv[0][0]])
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [60] for t in ts]; load[i] += 2 * est[p]
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
balance({(p, s): [(p, "cand1114", s), (p, "pint", s)] if s % 2 else [(p, "pint", s), (p, "cand1114", s)] for p in DUP for s in (1, 2, 3, 4)}, ND, "shards_D.json")
balance({(p, s): [(p, "cand1114", s), (p, "pint", s)] if s % 2 else [(p, "pint", s), (p, "cand1114", s)] for p in PROBS for s in (1, 2)}, NP, "shards_P.json")
