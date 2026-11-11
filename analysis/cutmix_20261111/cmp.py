"""設定どうしの比べ: 種ごとと全体の SGM の比 (ずらし 10 秒)・解けた回の数・問題ごとの比の 95% 区間 (問題の bootstrap、2000 回)・
種をまたいで続けて遅い/速い問題。
使い方: python cmp.py OUT.jsonl 基準 比べる設定,... [TL=60]"""
import json, math, random, sys
from collections import defaultdict
F, BASE, CMPS = sys.argv[1], sys.argv[2], sys.argv[3].split(",")
TL = float(sys.argv[4]) if len(sys.argv) > 4 else 60.0
res = defaultdict(dict)
for l in open(F):
    if l.strip():
        r = json.loads(l)
        ok = r["result"].get("status") == "optimal"
        res[(r["problem"], r["cfg"])][r["rep"]] = (min(r["result"].get("time", TL), TL) if ok else TL, ok)
for c in CMPS:
    probs = sorted({p for p, cc in res if cc == c and (p, BASE) in res})
    lr = {}  # 問題 -> 種 -> log 比
    sa = sb = 0
    for p in probs:
        a, b = res[(p, BASE)], res[(p, c)]
        ks = sorted(set(a) & set(b))
        lr[p] = {k: math.log((b[k][0] + 10) / (a[k][0] + 10)) for k in ks}
        sa += sum(a[k][1] for k in ks); sb += sum(b[k][1] for k in ks)
    seeds = sorted({k for d in lr.values() for k in d})
    per_seed = []
    for s in seeds:
        v = [d[s] for d in lr.values() if s in d]
        per_seed.append(f"種{s} {100 * (math.exp(sum(v) / len(v)) - 1):+.1f}%")
    pm = [sum(d.values()) / len(d) for d in lr.values() if d]
    mean = sum(pm) / len(pm)
    rnd = random.Random(1)
    bs = sorted(sum(rnd.choice(pm) for _ in pm) / len(pm) for _ in range(2000))
    lo, hi = bs[50], bs[1949]
    print(f"## {c} 対 {BASE} ({len(pm)} 問 × {len(seeds)} 種)")
    print(f"- SGM の比 {100 * (math.exp(mean) - 1):+.1f}% (95% 区間 {100 * (math.exp(lo) - 1):+.1f}% 〜 {100 * (math.exp(hi) - 1):+.1f}%)、" + "、".join(per_seed))
    print(f"- 解けた回 {BASE} {sa}、{c} {sb}")
    slow = [(p, d) for p, d in lr.items() if len(d) >= 2 and all(v > math.log(1.1) for v in d.values())]
    fast = [(p, d) for p, d in lr.items() if len(d) >= 2 and all(v < -math.log(1.1) for v in d.values())]
    fmt = lambda xs: ", ".join(f"{p} (" + "/".join(f"{100 * (math.exp(v) - 1):+.0f}%" for v in d.values()) + ")" for p, d in sorted(xs, key=lambda x: -abs(sum(x[1].values()))))
    print(f"- どの種でも 10% 超遅い: {fmt(slow) or 'なし'}")
    print(f"- どの種でも 10% 超速い: {fmt(fast) or 'なし'}")
