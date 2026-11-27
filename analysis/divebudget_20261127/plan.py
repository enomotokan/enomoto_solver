"""divebudget の P の分担表 (alns_20261126/plan.py を 3 設定にした形): python plan.py → shards_P.json (4 スレッド: def・db・ub) と
shards_P1.json (1 スレッド: def_t1・db_t1・ub_t1)。MIPLIB の 58 問 × 種 1〜3、60 秒、どちらも分担 1 つ (P は FMjE、P1 は FVbj)。
設定の順は問題・種で回す。見積りは analysis/final_20261118 の P の def の時間 + 8 秒。"""
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
# shards_P.json (def・db・ub) は FMjE が回している最中なので書き直さない。P1 は ubg_t1 を足した 4 設定、Pg は ubg (4 スレッド) だけ
for out, cf in (("shards_P1.json", ["def_t1", "db_t1", "ub_t1", "ubg_t1"]), ("shards_Pg.json", ["ubg"])):
    n = len(cf)
    jobs = [[p, cf[(i + s + j) % n], s, 60] for i, p in enumerate(PROBS) for s in (1, 2, 3) for j in range(n)]
    est = sum(T.get((p, s), T.get((p, 1), 60)) + 8 for p, _, s, _ in jobs)
    (D / out).write_text(json.dumps({"shards": [jobs], "est_hours": [round(est / 3600, 2)]}, indent=0))
    print(out, len(jobs), "回", "見積り (時間):", round(est / 3600, 2))
