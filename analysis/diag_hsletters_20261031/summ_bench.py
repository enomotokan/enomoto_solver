"""bench.jsonl (58 問、def・hs_pi0_bgw・hs_pi0_bgw_r) の集計。60 秒を超えて返った回は時間切れとして数える。
回ごとと全回で SGM (ずらし 10 秒)・解けた数、組 (負けている 21 問・MIPLIB 40 問) ごとの SGM、片方だけ解けた回、大きく差が出た問題、食い違いを出す。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0] / "cbcimpl_20261029"))
from probs import PROBS, SETS  # noqa: E402
CF = ["def", "hs_pi0_bgw", "hs_pi0_bgw_r"]
R = defaultdict(dict)
for l in open(here / "bench.jsonl"):
    r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
def ok(res): return res.get("status") == "optimal" and res.get("time", 99) <= 60.0
def t(res): return res["time"] if ok(res) else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
def row(keys, name):
    keys = [k for k in keys if all(c in R[k] for c in CF)]
    if not keys: return
    s = " | ".join(f"{sgm([t(R[k][c]) for k in keys]):.2f} ({sum(ok(R[k][c]) for k in keys)})" for c in CF)
    print(f"| {name} | {len(keys)} | {s} |")
reps = sorted({k[1] for k in R})
print("| 組 | 問題数 | " + " | ".join(CF) + " |\n|---|---|---|---|---|")
for rep in reps:
    for sn, ps in [("負けている 21 問", SETS["loss21"]), ("MIPLIB 40 問", SETS["miplib40"]), ("全 58 問", PROBS)]:
        row([(p, rep) for p in ps], f"回 {rep} {sn}")
row([(p, r) for p in PROBS for r in reps], "全回 全 58 問")
print("\n片方だけ解けた回・差が 3 秒かつ 20% 以上 (hs_pi0_bgw と hs_pi0_bgw_r):")
f = lambda r: f"{r['time']:.1f}" if ok(r) else "T"
for k in sorted(R):
    d = R[k]
    if all(c in d for c in CF):
        a, b = d[CF[1]], d[CF[2]]
        if ok(a) != ok(b) or abs(t(a) - t(b)) > max(3, 0.2 * min(t(a), t(b))):
            print(f"  {k[0]:28s} 回 {k[1]}  " + "  ".join(f"{c} {f(d[c])}" for c in CF))
print("\n60 秒超え・異常・食い違い:")
for k, d in sorted(R.items()):
    for c, v in d.items():
        if v.get("time", 0) > 61 or v.get("status") not in ("optimal", "time_limit"):
            print("  ", k, c, v.get("status"), round(v.get("time", 0), 1), str(v.get("error", ""))[:80])
    objs = [d[c]["objective"] for c in CF if c in d and d[c].get("status") == "optimal"]
    if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])):
        print("   食い違い", k, objs)
