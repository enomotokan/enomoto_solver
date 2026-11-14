"""設定どうしの揺れと平均の比べ: python summ2.py スレッド数 基準 設定... [-p 問題,...]
res_<設定>_t<スレッド>.jsonl と res_<設定>_t<スレッド>_<タグ>.jsonl (pair.py の分担) を読み、両方にある (問題, 並べ替え, 種) の回だけで比べる。
- 解けた回の数、問題ごとの最大/最小 (1 秒を足した時間の比、1 回でも解けて最大が 1 秒を超える問題)、gsd (1 秒を足した時間の対数の標準偏差の exp)
- SGM (ずらし 10 秒、時間切れは上限) の比と、問題で重みを揃えたブートストラップの 95% 区間
- 最遅の回 (問題ごとの最大) の SGM の比
誤答: 解けた回の目的値が同じ問題の中で 1e-6 (相対) を超えてずれたら印を付ける。"""
import json, math, random, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
a = sys.argv[1:]; only = None
if "-p" in a: i = a.index("-p"); only = set(a[i + 1].split(",")); del a[i:i + 2]
tags = None  # -t タグ,...: pair.py の分担の結果のうち、このタグのものだけを読む (例: -t s0f,s1f,s2f,s3f)
if "-t" in a: i = a.index("-t"); tags = a[i + 1].split(","); del a[i:i + 2]
TH, BASE, CFGS = a[0], a[1], a[2:]
def load(c):
    R = {}
    lines = [l for f in sorted(D.glob(f"res_{c}_t{TH}.jsonl")) + sorted(f for f in D.glob(f"res_{c}_t{TH}_*.jsonl") if tags is None or f.stem.split(f"_t{TH}_", 1)[1] in tags) for l in f.read_text().splitlines()]
    for l in lines:
        if not l.strip(): continue
        r = json.loads(l)
        if only and r["problem"] not in only: continue
        R[(r["problem"], r["perm"], r["seed"])] = r
    return R
def t_of(r):
    res = r["result"]; tl = r["tl"]
    ok = res.get("status") == "optimal"
    return (min(res.get("time") or tl, tl) if ok else tl), ok, res.get("objective")
B = load(BASE)
def gsd(ts):
    l = [math.log(1 + t) for t in ts]; m = sum(l) / len(l)
    return math.exp(math.sqrt(sum((x - m) ** 2 for x in l) / len(l)))
for c in CFGS:
    C = load(c)
    keys = sorted(set(B) & set(C))
    probs = sorted({k[0] for k in keys})
    print(f"## {c} 対 {BASE} ({TH} スレッド、{len(keys)} 回、{len(probs)} 問)")
    print("| 問題 | 解けた 基準/試作 | 最大/最小 基準/試作 | gsd 基準/試作 | 最遅 基準/試作 | 時間 (基準) | 時間 (試作) |")
    print("|---|---|---|---|---|---|---|")
    per = {}; wrong = []; mm = {BASE: [], c: []}
    for p in probs:
        ks = [k for k in keys if k[0] == p]
        row = {}
        for name, R in ((BASE, B), (c, C)):
            ts, oks, objs = zip(*[t_of(R[k]) for k in ks])
            row[name] = (ts, oks)
            good = [o for o, ok in zip(objs, oks) if ok and o is not None]
            per.setdefault(p, {})[name] = ts
        allobj = [t_of(R[k])[2] for R in (B, C) for k in ks if t_of(R[k])[1] and t_of(R[k])[2] is not None]
        if allobj and max(allobj) - min(allobj) > 1e-6 * max(1, abs(max(allobj))): wrong.append((p, min(allobj), max(allobj)))
        def mr(ts, oks): return (1 + max(ts)) / (1 + min(ts)) if any(oks) and max(ts) > 1 else None
        rb, rc = mr(*row[BASE]), mr(*row[c])
        if rb is not None and rc is not None: mm[BASE].append(rb); mm[c].append(rc)
        f = lambda x: "—" if x is None else f"{x:.2f}"
        print(f"| {p} | {sum(row[BASE][1])}/{sum(row[c][1])} | {f(rb)}/{f(rc)} | {gsd(row[BASE][0]):.2f}/{gsd(row[c][0]):.2f} | {max(row[BASE][0]):.1f}/{max(row[c][0]):.1f} | "
              + " ".join(f"{t:.1f}" for t in row[BASE][0]) + " | " + " ".join(f"{t:.1f}" for t in row[c][0]) + " |")
    def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
    def ratio(ps):
        num = [math.log(t + 10) for p in ps for t in per[p][c]]; den = [math.log(t + 10) for p in ps for t in per[p][BASE]]
        return math.exp(sum(num) / len(num) - sum(den) / len(den))
    rng = random.Random(0); bs = sorted(ratio([rng.choice(probs) for _ in probs]) for _ in range(2000))
    worst = lambda name: math.exp(sum(math.log(max(per[p][name]) + 10) for p in probs) / len(probs))
    nb = sum(t_of(B[k])[1] for k in keys); nc = sum(t_of(C[k])[1] for k in keys)
    gm = lambda v: math.exp(sum(math.log(x) for x in v) / len(v)) if v else float("nan")
    g_all = lambda name: gm([gsd(per[p][name]) for p in probs])
    print(f"\n- 解けた回: {nb} → {nc}")
    print(f"- SGM (ずらし 10 秒): {sgm([t for p in probs for t in per[p][BASE]]):.2f} → {sgm([t for p in probs for t in per[p][c]]):.2f} (比 {ratio(probs):.3f} [{bs[50]:.3f}, {bs[1949]:.3f}])")
    print(f"- 最遅の回の SGM の比: {worst(c) + 10 - 10:.2f} / {worst(BASE):.2f} = {(worst(c)) / (worst(BASE)):.3f}")
    print(f"- 最大/最小の幾何平均 ({len(mm[c])} 問): {gm(mm[BASE]):.3f} → {gm(mm[c]):.3f}; gsd の幾何平均: {g_all(BASE):.3f} → {g_all(c):.3f}")
    print(f"- 目的値のずれ: {wrong if wrong else 'なし'}\n")
