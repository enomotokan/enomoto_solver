"""buildopt の 4 スレッド P の照合: 食い違い (どちらも optimal で目的値が相対 1e-6 を超えて違う、または infeasible と optimal) と、ノード数の比 (参考、4 スレッドは非決定)。
使い方: python ana_P.py 既定 試作 jsonl..."""
import json, math, sys
from collections import defaultdict
base, proto = sys.argv[1], sys.argv[2]
by = defaultdict(dict)
for f in sys.argv[3:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l); by[r["problem"]][(r["cfg"], r["seed"])] = r["result"]
bad, nl, nsame, ntot = [], [], 0, 0
for p, d in sorted(by.items()):
    for s in (1, 2, 3):
        a, b = d.get((base, s)), d.get((proto, s))
        if not a or not b: continue
        sa, sb = a.get("status"), b.get("status")
        if {sa, sb} == {"optimal", "infeasible"}: bad.append((p, s, sa, sb))
        if sa == sb == "optimal":
            oa, ob = a["objective"], b["objective"]
            if abs(oa - ob) > 1e-6 * max(1, abs(oa), abs(ob)): bad.append((p, s, oa, ob))
            na, nb = a.get("nodes"), b.get("nodes")
            if na is not None and nb is not None:
                ntot += 1; nsame += na == nb; nl.append(math.log((nb + 1) / (na + 1)))
print(f"{proto}/{base}: 食い違い {len(bad)} 件", bad if bad else "")
if nl: print(f"  両方 optimal の {ntot} 回: ノード数が同じ {nsame} 回、(ノード+1) の比の幾何平均 {math.exp(sum(nl)/len(nl)):.3f}")
