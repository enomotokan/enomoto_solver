"""試作 `ENOMOTO_MIP_PCL` の比べ (python pcx.py res_PCX.jsonl)。1 行 = (問題, 種)、設定ごとのノード数と秒 (時間切れは *)。
まとめ: 設定ごとに def とのノード数の比・時間の比の幾何平均 (両方とも解けた回)、ノード数が 2 割以上減った回・増えた回。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
order = []
for l in open(D / sys.argv[1]):
    r = json.loads(l)
    k = (r["problem"], r["seed"])
    if k not in order:
        order.append(k)
    R[k][r["cfg"]] = r["result"]
cfgs = ["def"] + sorted({c for d in R.values() for c in d} - {"def"})
print(f"{'problem':22s} sd " + " ".join(f"{c:>16s}" for c in cfgs))
for k in order:
    d = R[k]
    cell = lambda x: f"{x.get('nodes') or 0:8d} {x.get('time') or 0:6.1f}{'*' if x.get('status') != 'optimal' else ' '}"
    print(f"{k[0]:22s} {k[1]:2d} " + " ".join(cell(d[c]) if c in d else f"{'-':>16s}" for c in cfgs))
for c in cfgs[1:]:
    ln, lt, b, w, n = [], [], 0, 0, 0
    for k in order:
        d = R[k]
        if c in d and "def" in d and d[c].get("status") == "optimal" and d["def"].get("status") == "optimal":
            rn = math.log((d[c]["nodes"] + 1) / (d["def"]["nodes"] + 1))
            ln.append(rn); lt.append(math.log((d[c]["time"] + 1) / (d["def"]["time"] + 1)))
            b += rn < math.log(0.8); w += rn > math.log(1.25)
    if ln:
        print(f"{c}: 両方解けた {len(ln)} 回、ノード数の比 {math.exp(sum(ln) / len(ln)):.3f}、時間の比 {math.exp(sum(lt) / len(lt)):.3f}、2 割以上減 {b}・2.5 割以上増 {w}")
