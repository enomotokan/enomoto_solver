"""SCIP-SYM の分担表: python plan.py → shards_P.json・shards_M.json
P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, pplex}、60 秒・4 スレッド、4 分担。
   PPLEX が働くのは physiciansched6-2 だけ (pplexscan.jsonl) なので、残りは揺れの大きさの目安。
M: PPLEX が働く Mittelmann の 5 問 (cod105・brazil3・mzzv11・glass4・nursesched-sprint02) × 種 1・2 × {def, pplex, def_t1, pplex_t1}、600 秒、4 分担。
同じ (問題, 種) の組は同じ分担で続けて解き、順は種で入れ替える。見積り: P は vshighs_20261117 の def の時間 + 8 秒、M は上限 + 30 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
def order(p, s, c):
    k = s % len(c)
    return [(p, x, s) for x in c[k:] + c[:k]]
T = defaultdict(list)
for f in glob.glob(str(D.parent / "vshighs_20261117" / "res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
TP = {k: sum(v) / len(v) for k, v in T.items()}
balance({(p, s): order(p, s, ["def", "pplex"]) for p in PROBS for s in (1, 2, 3)}, lambda p: TP.get(p, 48.0) + 8, 4, "shards_P.json", 60)
MP = ["cod105", "brazil3", "mzzv11", "glass4", "nursesched-sprint02"]
balance({(p, s): order(p, s, ["def", "pplex", "def_t1", "pplex_t1"]) for p in MP for s in (1, 2)}, lambda p: 630.0, 4, "shards_M.json", 600)
