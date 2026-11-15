"""D2 の分担表を作る。python plan.py NP NM [M の設定 (既定 base,d2ab)] → shards_P.json・shards_M.json
(P) 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {base, d2a, d2b, d2ab} 60 秒。同じ (問題, 種) の 4 設定は同じ分担に入れる (時期・機械のずれを対で消す)。
(M) HIGHSLOSS の 12 問 (analysis/highsloss_20261113/shards.json) × 種 1 × M の設定 600 秒。同じ問題は同じ分担。
時間の見積りは cand1114 の 58 問の回 (analysis/cand1114_20261113/res_P_s*.jsonl の cand1114) と HIGHSLOSS の cand の回。"""
import glob, json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NP, NM = int(sys.argv[1]), int(sys.argv[2])
MCFG = (sys.argv[3] if len(sys.argv) > 3 else "base,d2ab").split(",")
PCFG = ["base", "d2a", "d2b", "d2ab"]
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -est(kv[0]) * len(kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for key, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += est(key) * len(ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
TP = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1114_20261113/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114":
            TP[r["problem"]].append(min(r["result"].get("time") or 60, 60))
gP = {(p, s): [(p, c, s) for c in PCFG] for p in PROBS for s in (1, 2, 3)}
balance(gP, lambda k: (sum(TP[k[0]]) / len(TP[k[0]]) if TP[k[0]] else 50) + 8, NP, "shards_P.json", 60)
HL = sorted({p for s in json.loads((WT / "analysis/highsloss_20261113/shards.json").read_text())["shards"] for p, _ in s})
TM = {}
for f in glob.glob(str(WT / "analysis/highsloss_20261113/res_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand":
            TM[r["problem"]] = min(r["result"].get("time") or 600, 600)
gM = {p: [(p, c, 1) for c in MCFG] for p in HL}
balance(gM, lambda p: TM.get(p, 600) + 40, NM, "shards_M.json", 600)
