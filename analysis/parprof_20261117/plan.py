"""分担表: python plan.py NP NM → shards_P.json (23 問 × 種 1・2 × 1・2・4 スレッド、120 秒、138 回) と shards_M.json (4 問 × 種 1・2 × 1・2・4 スレッド、600 秒、24 回)。
同じ問題・種の 3 つは同じ分担で続けて解く。見積りは cand1118_default の種 1 の def_t1 (1 スレッド)・def (4 スレッド) の時間 (時間切れは上限)。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from probs import P, M, THREADS, SEEDS, TL  # noqa: E402
T = {}
for f in glob.glob(str(D.parent / "cand1118_default_20261116/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); st = (r["result"].get("status") or "").lower()
        T[(r["problem"], r["cfg"])] = r["result"]["time"] if st in ("optimal", "infeasible") else None
def est(p, th, tl):
    t1, t4 = T.get((p, "def_t1")), T.get((p, "def"))
    t1 = tl if t1 is None else min(t1, tl); t4 = tl if t4 is None else min(t4, tl)
    return {1: t1, 2: (t1 + t4) / 2, 4: t4}[th] + 8 if p in P else tl * {1: 1.0, 2: 0.9, 4: 0.8}[th] + 30
def balance(probs, n, SET):
    tl = TL[SET]; load = [0.0] * n; shards = [[] for _ in range(n)]
    groups = [((p, s), [[p, th, s, tl] for th in THREADS]) for p in probs for s in SEEDS]
    for key, runs in sorted(groups, key=lambda kv: -sum(est(kv[0][0], r[1], tl) for r in kv[1])):
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += sum(est(key[0], r[1], tl) for r in runs)
    (D / f"shards_{SET}.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(f"shards_{SET}.json", n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
balance(P, int(sys.argv[1]), "P")
balance(M, int(sys.argv[2]), "M")
