"""DUALEQ2 の P の分担表: python plan.py → shards_P.json。MIPLIB の 58 問 × 種 1〜3、60 秒。
分担 0 = 1 スレッド (def_t1・deq_t1)、分担 1 = 4 スレッド (def・deq)。設定の順は問題・種で回す。見積りは final_20261118 の P の def の時間 + 8 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
T = {}
for f in glob.glob(str(D.parent / "final_20261118" / "res_P_s*.jsonl")):
    for l in open(f):
        if not l.strip(): continue
        r = json.loads(l)
        if r["cfg"] == "def" and "time" in r["result"]: T[(r["problem"], r["seed"])] = min(r["result"]["time"], 60) if solved(r) else 60
shards, est = [], []
for cf in (["def_t1", "deq_t1"], ["def", "deq"]):
    jobs = [[p, cf[(i + s + j) % 2], s, 60] for i, p in enumerate(PROBS) for s in (1, 2, 3) for j in range(2)]
    shards.append(jobs); est.append(round(sum(T.get((p, s), T.get((p, 1), 60)) + 8 for p, _, s, _ in jobs) / 3600, 2))
(D / "shards_P.json").write_text(json.dumps({"shards": shards, "est_hours": est}, indent=0))
print([len(s) for s in shards], "回", "見積り (時間):", est)
