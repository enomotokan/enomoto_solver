"""分担表: python plan.py NM NP → shards_M.json (2 問 × 10 設定 × 種 1・2、600 秒、40 回) と shards_P.json (6 問 × 10 設定 × 種 1・2、120 秒、120 回)。
見積りは parprof_20261117 の 4 スレッドの時間 (時間切れは上限) + 起動の分。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFG, M, P, SEEDS, TL  # noqa: E402
T = {}
for f in glob.glob(str(D.parent / "parprof_20261117/res_*_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["threads"] == 4:
            st = (r["result"].get("status") or "").lower()
            T[(r["problem"], r["seed"])] = r["result"]["time"] if st in ("optimal", "infeasible") else None
def est(p, s, tl):
    t = T.get((p, s)); return (tl if t is None else min(t * 1.5, tl)) + 15
def balance(probs, n, SET):
    tl = TL[SET]; load = [0.0] * n; shards = [[] for _ in range(n)]
    runs = [[p, c, s, tl] for p in probs for c in CFG for s in SEEDS]
    for r in sorted(runs, key=lambda r: -est(r[0], r[2], tl)):
        i = min(range(n), key=lambda j: load[j]); shards[i].append(r); load[i] += est(r[0], r[2], tl)
    (D / f"shards_{SET}.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(f"shards_{SET}.json", n, "分担、", len(runs), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
balance(M, int(sys.argv[1]), "M")
balance(P, int(sys.argv[2]), "P")
