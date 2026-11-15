"""res58.jsonl を設定ごとに集計する: 解けた数、SGM (ずらし 10 秒、解けなかった回は 60 秒)、両方最適で目的値が違う問題、片方だけ解けた問題、
時間の比が 1.5 倍を超えて違う問題、暫定解の比べ (両方時間切れ)。使い方: python summ58.py [設定A 設定B]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in open(D / "res58.jsonl"):
    x = json.loads(l)
    R[x["problem"]][x["cfg"]] = x["result"]
a, b = (sys.argv[1:3] if len(sys.argv) > 2 else ("def", "rr"))
ok = lambda r: r.get("status") in ("optimal", "Optimal", "infeasible", "Infeasible")
tm = lambda r: r["time"] if ok(r) and r.get("time") is not None else 60.0
probs = [p for p in R if a in R[p] and b in R[p]]
for c in (a, b):
    n = sum(ok(R[p][c]) for p in probs)
    sgm = math.exp(sum(math.log(tm(R[p][c]) + 10) for p in probs) / len(probs)) - 10
    w = sum(R[p][c].get("mip_warnings", 0) for p in probs)
    print(f"{c}: {len(probs)} 問、解けた {n}、SGM {sgm:.2f} 秒、WARNING {w}")
for p in probs:
    ra, rb = R[p][a], R[p][b]
    oa, ob = ra.get("objective"), rb.get("objective")
    if ok(ra) and ok(rb) and oa is not None and ob is not None and abs(oa - ob) > 1e-6 * max(1, abs(oa)):
        print("食い違い", p, oa, ob)
    if ok(ra) != ok(rb):
        print("片方だけ解けた", p, a, ra.get("status"), round(tm(ra), 1), b, rb.get("status"), round(tm(rb), 1))
    elif ok(ra) and ok(rb) and (tm(ra) + 10) / (tm(rb) + 10) > 1.5 or (tm(rb) + 10) / (tm(ra) + 10) > 1.5:
        print("時間の差", p, a, round(tm(ra), 1), b, round(tm(rb), 1))
    elif not ok(ra) and not ok(rb) and oa != ob:
        print("両方時間切れ", p, a, oa, ra.get("bound"), b, ob, rb.get("bound"))
