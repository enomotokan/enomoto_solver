"""設定ごとの強分岐の LP の数・反復、LP の反復、ノード数の比 (同じ (問題, 種) の対の幾何平均、主のスレッドの最後の進みの行) と、
全ての回の暫定解と下界の矛盾の検査 (最小化: どの回の下界も、どの回の暫定解を許容誤差を超えて上回らない)。使い方: python cmpstats.py OUT.jsonl..."""
import json, math, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
rows = []
for f in sys.argv[1:]:
    for l in Path(f).read_text().splitlines():
        if l.strip():
            r = json.loads(l); r["_stem"] = Path(f).stem; rows.append(r)
def st(r):
    log = D / "logs" / f"{r['_stem']}_{r['problem']}_{r['cfg']}_s{r['rep']}.log"
    d = {}
    if log.exists():
        for l in log.read_text().splitlines():
            m = re.search(r"lp_iters (\d+) sb_iters (\d+) sb_lps (\d+)", l)
            if m:
                d = dict(zip(("lp", "sbit", "sblp"), map(int, m.groups())))
    d["nodes"] = r["result"].get("nodes")
    return d
T = defaultdict(dict)
for r in rows:
    T[(r["_stem"], r["problem"], r["rep"])][r["cfg"]] = (r["result"], st(r))
cfgs = sorted({r["cfg"] for r in rows} - {"def"})
for c in cfgs:
    for key in ("sblp", "sbit", "lp", "nodes"):
        xs = []; xs_solved = []
        for k, v in T.items():
            if c in v and "def" in v and v[c][1].get(key) is not None and v["def"][1].get(key) is not None:
                a, b = v["def"][1][key], v[c][1][key]
                lr = math.log((b + 1) / (a + 1)); xs.append(lr)
                if v["def"][0].get("status") == "optimal" and v[c][0].get("status") == "optimal":
                    xs_solved.append(lr)
        f = lambda z: f"{(math.exp(sum(z)/len(z))-1)*100:+.1f}% (n {len(z)})" if z else "-"
        print(f"{c} {key}: 全部 {f(xs)}  両方解けた {f(xs_solved)}")
# 矛盾の検査
bad = 0; byp = defaultdict(list)
for r in rows:
    byp[r["problem"]].append(r)
for p, rs in byp.items():
    objs = [r["result"].get("objective") for r in rs if r["result"].get("objective") is not None]
    bnds = [r["result"].get("bound") for r in rs if r["result"].get("bound") is not None]
    if not objs or not bnds:
        continue
    lo, hi = min(objs), max(bnds)
    if hi > lo + 1e-6 * max(1.0, abs(lo)):
        bad += 1
        print("矛盾", p, "最大の下界", hi, "最小の暫定解", lo, [(r["cfg"], r["rep"], r["result"].get("status"), r["result"].get("objective"), r["result"].get("bound")) for r in rs])
    opt = [r for r in rs if r["result"].get("status") == "optimal"]
    vals = {round(r["result"]["objective"], 6) for r in opt}
    if len(opt) > 1 and max(vals) - min(vals) > 1e-6 * max(1.0, abs(min(vals))):
        bad += 1; print("最適値の不一致", p, vals)
print("矛盾の数", bad, "問題", len(byp))
