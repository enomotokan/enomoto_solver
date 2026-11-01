"""bench8 の集計 (今の既定 def と、サブ MIP のノード上限を下げる nm02・nm01)。summ7.py を元にした。
60 秒を超えて返った回は時間切れとして数える。58 問の回ごと・全回の SGM (ずらし 10 秒)・解けた数、片方だけ解けた回、異常を出す。
使い方: python summ8.py [比べる設定 (既定 nm02,nm01)]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
A = "def"
BS = sys.argv[1].split(",") if len(sys.argv) > 1 else ["nm02", "nm01"]
R = defaultdict(dict)
for f in sorted(here.glob("bench8_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
def ok(res): return res.get("status") == "optimal" and res.get("time", 99) <= 60.0
def t(res): return res["time"] if ok(res) else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
def row(keys, name, B):
    keys = [k for k in keys if A in R[k] and B in R[k]]
    if not keys:
        return
    sa, sb = sgm([t(R[k][A]) for k in keys]), sgm([t(R[k][B]) for k in keys])
    na, nb = sum(ok(R[k][A]) for k in keys), sum(ok(R[k][B]) for k in keys)
    print(f"{name:14s} n={len(keys):3d}  {A}: {sa:6.2f}s ({na})  {B}: {sb:6.2f}s ({nb})  比 {sb / sa - 1:+.1%}")
reps = sorted({k[1] for k in R})
for B in BS:
    print(f"== {A} と {B}")
    for rep in reps:
        row([(p, rep) for p in PROBS], f"58問 rep {rep}", B)
    row([(p, r) for p in PROBS for r in reps], "58問 全回", B)
    print("  片方だけ解けた回・2 倍以上 (5 秒以上) 違う回:")
    for k in sorted(R):
        d = R[k]
        if A in d and B in d and (ok(d[A]) != ok(d[B]) or (max(t(d[A]), t(d[B])) >= 5 and max(t(d[A]), t(d[B])) >= 2 * min(t(d[A]), t(d[B])))):
            f = lambda r: f"{r['time']:.1f}" if ok(r) else "TL"
            print(f"    {k[0]:28s} r{k[1]:3d}  {A} {f(d[A]):>6s}  {B} {f(d[B]):>6s}")
print("\n欠け (3 設定が揃っていない問題・回):", sorted(k for k, d in R.items() if len(d) < 1 + len(BS)))
print("60 秒超え・異常・食い違い:")
for k, d in sorted(R.items()):
    for c, v in d.items():
        if v.get("time", 0) > 61 or v.get("status") not in ("optimal", "time_limit"):
            print("  ", k, c, v.get("status"), round(v.get("time", 0), 1), str(v.get("error", ""))[:80])
    zs = [d[c]["objective"] for c in d if d[c].get("status") == "optimal"]
    if zs and max(zs) - min(zs) > 1e-6 * max(1, abs(zs[0])):
        print("   食い違い", k, zs)
