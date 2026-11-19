"""根の強分岐を強める試作 `ENOMOTO_MIP_ROOTSB` の根だけの比べ (python rsbana.py res_RSB_root.jsonl)。
1 回 = (問題, 種, 設定)。分枝の上がり幅 = (min(分枝した列の子 2 つ) - z) / G。子は `SBCHOSEN` (反復の上限なしで解き直した値、どの設定でも
同じ物差し)、z はその根の LP の値、G = 参照値 (solu) - z。強分岐の反復 = 根の分枝の選択の終わりの sb_iters + rootsb_iters、
秒 = 根だけの求解の時間 (ノード上限 1)。"""
import json, sys, statistics
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
solu = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] in ("=opt=", "=best="):
        solu[f[1]] = float(f[2])
R = defaultdict(dict)
order = []
for fn in sys.argv[1:]:
    for l in open(D / fn):
        r = json.loads(l)
        main = [s for s in r["cutstop"] if not s["bg"]]
        sense = main[0]["sense"] if main else 1
        ref = solu.get(r["problem"])
        ch = r.get("sbchosen") or []
        end = [e for e in r["sbend"] if e["action"] == "branch"]
        row = {"time": r["result"].get("time"), "status": r["result"].get("status")}
        if ch and ref is not None:
            c = ch[0]
            z = c["z"]
            G = sense * ref - z
            vals = [v for v in (c["down"], c["up"]) if isinstance(v, (int, float))]
            m = min(min(vals) if vals else sense * ref, sense * ref)
            row.update(gain=(m - z) / G if G > 1e-9 else None, col=c["col"], G=G)
        if end:
            row["sbit"] = end[0]["sb_iters"] + end[0]["rootsb_iters"]
            row["sb"] = end[0]["sb"]
        row["neval"] = len(r["sbcand"])
        key = (r["problem"], r["seed"])
        if key not in order:
            order.append(key)
        R[key][r.get("cfg", "def")] = row
cfgs = sorted({c for d in R.values() for c in d}, key=lambda c: (c != "def", c))
f = lambda v: "      -" if v is None else f"{v:7.4f}"
print(f"{'problem':22s} sd " + " | ".join(f"{c + ' gain':>10s} {'eval':>4s} {'sbit':>7s} {'秒':>6s}" for c in cfgs))
acc = defaultdict(list)
for key in order:
    d = R[key]
    print(f"{key[0]:22s} {key[1]:2d} " + " | ".join(f"{f(d[c].get('gain')):>10s} {d[c].get('neval', 0):4d} {d[c].get('sbit', 0):7d} {d[c]['time'] or 0:6.1f}" if c in d else f"{'-':>10s} {'':4s} {'':7s} {'':6s}" for c in cfgs))
    if all(c in d and d[c].get("gain") is not None for c in cfgs):
        for c in cfgs:
            acc[c].append((d[c]["gain"], d[c]["time"] or 0, d[c].get("sbit", 0)))
print()
for c in cfgs:
    v = acc[c]
    if v:
        print(f"{c}: {len(v)} 回、上がり幅の平均 {statistics.mean(x[0] for x in v):.4f}・中央値 {statistics.median(x[0] for x in v):.4f}、根の秒の合計 {sum(x[1] for x in v):.0f}、強分岐の反復の中央値 {statistics.median(x[2] for x in v):.0f}")
if "def" in cfgs:
    for c in cfgs:
        if c == "def":
            continue
        b = w = s = 0
        for key in order:
            d = R[key]
            if c in d and "def" in d and d[c].get("gain") is not None and d["def"].get("gain") is not None:
                dd = d[c]["gain"] - d["def"]["gain"]
                b += dd > 1e-3; w += dd < -1e-3; s += abs(dd) <= 1e-3
        print(f"{c} と def: 上がり幅が良い {b}・悪い {w}・同じ {s} (差 1e-3 で)")
