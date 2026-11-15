"""根の比べの表 (python rootsumm.py root58.jsonl): 閉じたギャップ = (根の下界 - 最初の LP) / (最適値 - 最初の LP)。
根の下界は ENOMOTO はカットのループの終わりの LP の値 (3 つ目の引数 first (既定) なら最初のループ = 再開の前、final ならループの最大 = 再開の後を含む)、HiGHS は mip_dual_bound (ノード上限 1)。
最初の LP は既定の根のループの obj0、最適値は scripts/miplib2017-v31.solu (なければ記録の中の最良の解)。
時間は ENOMOTO は根のループの終わりの時刻 (t) と全体 (ノード上限 1、ヒューリスティクスを含む)、HiGHS は全体"""
import json, sys, math
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
for l in open(D / sys.argv[1]):
    r = json.loads(l)
    if r["problem"] not in order:
        order.append(r["problem"])
    R[r["problem"]][r["cfg"]] = r
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 and sys.argv[2] != "-" else ["def", "a", "b", "highs"]
# first: 最初の根のループ (再開の前) の終わりの値 (カットの効きだけを見る)。final: ループの最大 (再開の後を含む)
MODE = sys.argv[3] if len(sys.argv) > 3 else "first"
print(f"{'problem':22s} " + " ".join(f"{c:>8s}" for c in cfgs) + "  | t_loop " + " ".join(f"{c:>6s}" for c in cfgs if c != "highs") + " | t_all " + " ".join(f"{c:>6s}" for c in cfgs))
acc = defaultdict(list)
cmp = defaultdict(lambda: [0, 0, 0])
for p in order:
    d = R[p]
    if "def" not in d or not d["def"].get("root_loops"):
        continue
    lp0 = d["def"]["root_loops"][0]["obj0"]
    opt = solu.get(p)
    if opt is None:
        objs = [x["result"].get("objective") for x in d.values() if x["result"].get("objective") is not None]
        opt = min(objs) if objs else None
    if opt is None or abs(opt - lp0) <= 1e-6 * max(1, abs(opt)):
        continue
    g = {}
    for c in cfgs:
        if c in d:
            if c == "highs":
                rb = d[c]["result"].get("bound")
            else:
                lps = d[c].get("root_loops") or []
                rb = (lps[0]["obj1"] if MODE == "first" else max(x["obj1"] for x in lps)) if lps else None
            g[c] = None if rb is None or not math.isfinite(rb) else max(-1.0, min(1.0, (rb - lp0) / (opt - lp0)))
    if all(c in g and g[c] is not None for c in cfgs):
        for c in cfgs:
            acc[c].append(g[c])
        for c in cfgs:
            if c != "def":
                dd = g[c] - g["def"]
                cmp[c][0 if dd > 1e-3 else (1 if dd < -1e-3 else 2)] += 1
    tl = [f"{d[c]['root_loops'][0]['t'] or 0:6.2f}" if c in d and d[c].get("root_loops") else "     -" for c in cfgs if c != "highs"]
    ta = [f"{d[c]['result'].get('time') or 0:6.1f}" if c in d else "     -" for c in cfgs]
    print(f"{p:22s} " + " ".join(f"{g[c]:8.4f}" if g.get(c) is not None else "       -" for c in cfgs) + "  |        " + " ".join(tl) + " |       " + " ".join(ta))
print(f"\n{len(acc['def'])} 問 (ギャップ 0 でないもの、全設定がそろうもの) の閉じたギャップの平均: " + ", ".join(f"{c} {sum(v)/len(v):.4f}" for c, v in acc.items() if v))
for c, (b, w, s) in cmp.items():
    print(f"  {c} と def: 良い {b}・悪い {w}・同じ {s} (差 1e-3 で)")
