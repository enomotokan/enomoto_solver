"""仕上げの対 (4T、cand1114 と cand1114 + ENOMOTO_MIP_ROBUST=pgq、58 問 × 種 1〜4) を分担に分ける: python plan.py N > shards.json
同じ (問題, 種) の 2 組は同じ分担で交互に解く。見込み時間は ../res_t4_base2p.jsonl・../res_t4_pgq.jsonl の並べ替え 0 の時間 (種 4 は種 1 の値、時間切れは 60 秒) + 1 回 3 秒。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1])
t = {}
for f in ("res_t4_base2p.jsonl", "res_t4_pgq.jsonl"):
    for l in (D.parent / f).read_text().splitlines():
        r = json.loads(l); res = r["result"]
        if r["perm"] != 0: continue
        x = min(res.get("time") or 60, 60) if res.get("status") == "optimal" else 60
        t[(r["problem"], r["seed"])] = t.get((r["problem"], r["seed"]), 0) + x + 3
for (p, s) in list(t):
    if s == 1: t[(p, 4)] = t[(p, 1)]
keys = sorted(t, key=lambda k: (-t[k], k))
load = [0.0] * N; shards = [[] for _ in range(N)]
for k in keys:
    i = min(range(N), key=lambda i: load[i]); load[i] += t[k]
    shards[i] += [[k[0], k[1], c] for c in ("cand1114", "pgq")]
print(json.dumps({"n": N, "est_min": [round(x / 60, 1) for x in load], "shards": shards}))
