"""再スタートの起きる問題の比べ (analysis/restchain_20261111.md): python summ_restart.py res58.jsonl 設定[,設定]
どれかの設定で再スタートが起きた (問題, 回) について、設定ごとに 再スタートの回数・各再スタートで新たに固定した整数列の数と
その時点の暫定解の値・解く時間 (解けなければ時間上限)・最後の目的値と下界を並べる。"""
import json, sys
from collections import defaultdict
rows = defaultdict(dict)
for l in open(sys.argv[1]):
    r = json.loads(l)
    rows[(r["problem"], r["rep"])][r["cfg"]] = r
cfgs = ["def"] + sys.argv[2].split(",")
def t(r):
    res = r["result"]
    return res.get("time") if res.get("status") in ("optimal", "infeasible") else None
n = 0
for k in sorted(rows):
    rr = rows[k]
    if not all(c in rr for c in cfgs) or not any(rr[c].get("restarts") for c in cfgs if "restarts" in rr[c]):
        continue
    n += 1
    print(f"{k[0]} 種 {k[1]}")
    for c in cfgs:
        r = rr[c]
        res = r["result"]
        rs = r.get("restarts") or []
        tt = t(r)
        desc = " ".join(f"[{a}:{b}固定,inc={d if d is None else f'{d:.6g}'}]" for a, b, _, d in rs)
        print(f"  {c:5s} 時間 {('%.1f' % tt) if tt is not None else '  -  '} 目的 {res.get('objective')} 下界 {res.get('bound')} 再スタート {len(rs)} 立て直し {r.get('respawn', 0)} {desc}")
print(f"{n} 回")
