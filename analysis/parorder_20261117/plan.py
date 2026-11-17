"""対の分担表: python plan.py NM NP → shards_M.json (重い 4 問 × PM の設定 × 種 1・2、600 秒) と shards_P.json (58 問 × PM の設定 × 種 1〜3、60 秒)。
同じ (問題, 種) の全設定を同じ分担 (= 同じ機械) に入れる。設定ごとに分担を分けない。
見積りは parprof_20261117 の 4 スレッドの時間 (解けた問題は 1.5 倍、時間切れと記録のない問題は上限) + 起動の 15 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from cfg import PM  # noqa: E402
from probs import PROBS  # noqa: E402
M = ["neos-3627168-kasai", "csched008", "sct2", "tbfp-network"]
SEEDS = {"M": [1, 2], "P": [1, 2, 3]}
TL = {"M": 600, "P": 60}
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
    groups = [[[p, c, s, tl] for c in PM] for p in probs for s in SEEDS[SET]]
    for g in sorted(groups, key=lambda g: -est(g[0][0], g[0][2], tl)):
        i = min(range(n), key=lambda j: load[j]); shards[i] += g; load[i] += sum(est(r[0], r[2], tl) for r in g)
    (D / f"shards_{SET}.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(f"shards_{SET}.json", n, "分担、", sum(len(x) for x in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
balance(M, int(sys.argv[1]), "M")
balance(PROBS, int(sys.argv[2]), "P")
