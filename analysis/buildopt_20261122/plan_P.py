"""buildopt の 4 スレッド P の分担表: python plan_P.py NS [版 ...] → shards_P.json (MIPLIB の 58 問 × 種 1〜3 × 版、60 秒)。
版の既定は base pgo_v3 (var/NAME)。版の順は問題・種で回す。見積りは analysis/final_20261118 の P の def の時間 + 8 秒。gainstall_20261122/plan.py の P の写し。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NS = int(sys.argv[1]); CF = sys.argv[2:] or ["base", "pgo_v3"]
for c in CF: assert (D / "var" / c / "enomoto_solver").is_dir(), c
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
TP = {}
for f in glob.glob(str(D.parent / "final_20261118" / "res_P_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l)
            if r["cfg"] == "def" and "time" in r["result"]: TP[(r["problem"], r["seed"])] = min(r["result"]["time"], 60) if solved(r) else 60
est = lambda p, c, s: TP.get((p, s), TP.get((p, 1), 60)) + 8
n = len(CF)
groups = {(p, s): [(p, c, s) for c in (CF[(i + s) % n:] + CF[:(i + s) % n])] for i, p in enumerate(PROBS) for s in (1, 2, 3)}
gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
load = [0.0] * NS; shards = [[] for _ in range(NS)]
for _, ts in gl:
    i = min(range(NS), key=lambda j: load[j]); shards[i] += [list(t) + [60] for t in ts]; load[i] += sum(est(*t) for t in ts)
(D / "shards_P.json").write_text(json.dumps({"cfgs": CF, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_P.json", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load], "計", round(sum(load) / 3600, 1))
