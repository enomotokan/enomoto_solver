"""SCIP-VB の分担表: python plan.py P NP → shards_P.json (MIPLIB の 58 問 × 種 1〜3 × {def, vb}、60 秒)。
python plan.py M NM → shards_M.json (count_M_b.jsonl で VB01 がサブ MIP まで行った Mittelmann の問題 × 種 1・2 × {def, vb}、600 秒)。
見積りは analysis/final_20261118 の def の時間 (解けなかった回は上限) + P は 8 秒・M は 30 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
FIN = D.parent / "final_20261118"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
SET, NS = sys.argv[1], int(sys.argv[2])
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = {}
    for f in glob.glob(str(FIN / pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] == "def": T[(r["problem"], r["seed"])] = min(r["result"]["time"], cap) if solved(r) else cap
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
if SET == "P":
    CF = ["def", "vb"]; TP = times("res_P_s*.jsonl", 60)
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
            lambda p, c, s: TP.get((p, 1), 60) + 8, NS, "shards_P.json", 60)
else:
    CF = ["def", "vb"]; TM = times("res_M_s*.jsonl", 600)
    work = sorted(r["problem"] for l in open(D / "count_M_b.jsonl") if l.strip() for r in [json.loads(l)] if r.get("nsub", 0) > 0)
    print("M:", len(work), "問")
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(work) for s in (1, 2)},
            lambda p, c, s: TM.get((p, 1), 600) + 30, NS, "shards_M.json", 600)
