"""4 スレッドの 2 組 (既定 base2・spgq) の 58 問 × 6 変種を分担に分ける: python plan.py N [組A 組B] > shards[_組B].json
同じ (問題, 変種) の 2 組は同じ分担。見込み時間は ../res_t4_base.jsonl・../res_t4_spf.jsonl の時間 (時間切れは 60 秒) + 1 回 3 秒。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1])
PAIR = tuple(sys.argv[2:4]) if len(sys.argv) > 3 else ("base2", "spgq")
est = {}
for f in ("res_t4_base.jsonl", "res_t4_spf.jsonl"):
    for l in (D.parent / f).read_text().splitlines():
        r = json.loads(l); res = r["result"]
        t = min(res.get("time") or 60, 60) if res.get("status") == "optimal" else 60
        est[(r["problem"], r["perm"], r["seed"])] = est.get((r["problem"], r["perm"], r["seed"]), 0) + t + 3
keys = sorted(est, key=lambda k: -est[k])
load = [0.0] * N; shards = [[] for _ in range(N)]
for k in keys:
    i = min(range(N), key=lambda i: load[i]); load[i] += est[k]
    shards[i] += [[k[0], k[1], k[2], c] for c in PAIR]
print(json.dumps({"n": N, "est_min": [round(x / 60, 1) for x in load], "shards": shards}))
