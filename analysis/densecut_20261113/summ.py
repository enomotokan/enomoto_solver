"""densecut の集計: python summ.py RES.jsonl BASE ALT → 問題ごとの診断 (BASE の回、スレッドの和) と ALT/BASE の時間比、SGM (shift 10) の区間。"""
import json, math, re, sys, random
from collections import defaultdict
res, base, alt = sys.argv[1], sys.argv[2], sys.argv[3]
R = defaultdict(dict)
for l in open(res):
    r = json.loads(l); R[r["problem"], r["seed"]][r["cfg"]] = r
def num(s, key):
    m = re.search(re.escape(key) + r" ([-0-9.e]+)", s); return float(m.group(1)) if m else 0.0
def agg(r):
    a = defaultdict(float); hist = [0] * 5
    for s in r["dc"]:
        lps = num(s, " lps"); a["lps"] += lps; a["nodes"] += num(s, " nodes")
        a["rows"] += num(s, "rows/lp") * lps; a["post"] += num(s, "post rows/lp") * lps; a["postnz"] += num(s, "post nnz/lp") * lps
        a["lpsecs"] += num(s, "lpsecs"); a["cutsecs"] += num(s, "cutsecs")
        nc = num(s, "ncuts"); a["ncuts"] += nc; a["nnz"] += num(s, "nnz/cut") * nc
        a["rowavg"] = num(s, "rowavg"); a["m"] = num(s, " m"); a["orignnz"] = num(s, "orignnz")
        gn = num(s, "gain n"); a["gn"] += gn; a["gsum"] += num(s, "mean") * gn
        a["dd"] += num(s, "dense depth"); a["dh"] += num(s, "dense hits")
        h = re.search(r"hist \[([0-9, ]+)\]", s)
        if h: hist = [x + int(y) for x, y in zip(hist, h.group(1).split(","))]
    a["hist"] = hist
    return a
def t(r):
    x = r["result"]; tl = r["tl"]
    return (x.get("time") or tl) if x.get("status") == "optimal" else tl
rows = []; ratios = defaultdict(list)
for (p, s), c in sorted(R.items()):
    if base not in c or alt not in c: continue
    a = agg(c[base]); tb, ta = t(c[base]), t(c[alt])
    ratios[p].append((ta + 10) / (tb + 10))
    lps = max(a["lps"], 1)
    rows.append((p, s, tb, ta, a["ncuts"], a["nnz"] / max(a["ncuts"], 1), a["rowavg"], a["post"] / lps, a["postnz"] / lps / max(a["orignnz"], 1), a["m"],
                 a["lpsecs"] / lps * 1000, a["cutsecs"], a["gsum"] / max(a["gn"], 1), a["gn"], a["hist"], a["dd"], a["dh"], int(a["nodes"]), c[base]["result"].get("status"), c[alt]["result"].get("status")))
print(f"{'問題':24s} 種 {base:>6s} {alt:>6s} 比   カット 非零/本 元の行 比率 後の行/LP 後の非零/元 m      ms/LP  分離秒 伸び平均 測った回 密深さ 密判定 ノード")
for r in rows:
    p, s, tb, ta, nc, nz, ra, post, pz, m, ms, cs, g, gn, h, dd, dh, nd, sb, sa = r
    print(f"{p:24s} {s} {tb:6.1f}{'*' if sb!='optimal' else ' '}{ta:6.1f}{'*' if sa!='optimal' else ' '} {(ta+10)/(tb+10):4.2f} {nc:6.0f} {nz:7.1f} {ra:6.1f} {nz/max(ra,1e-9):5.1f} {post:7.1f} {pz:6.3f} {m:6.0f} {ms:7.2f} {cs:6.2f} {g:7.3f} {gn:6.0f} {dd:5.0f} {dh:6.0f} {nd}")
lr = [math.log(x) for v in ratios.values() for x in [math.exp(sum(map(math.log, v)) / len(v))]]
def sgm(l): return math.exp(sum(l) / len(l)) - 1
random.seed(0)
bs = sorted(sgm([random.choice(lr) for _ in lr]) for _ in range(2000))
print(f"SGM {alt}/{base} ({len(lr)} 問、種で平均してから): {sgm(lr)*100:+.1f}% [{bs[50]*100:+.1f}%, {bs[1949]*100:+.1f}%]")
