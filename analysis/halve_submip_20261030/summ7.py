"""bench7 の集計 (def と既定にする候補 cand)。60 秒を超えて返った回は時間切れとして数える。
58 問は回ごと、Mittelmann (rep 102) はまとめて、SGM (ずらし 10 秒)・解けた数・勝ち負けを出す。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
A, B = "def", "cand"
R = defaultdict(dict)
for f in sorted(here.glob("bench7_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] in (A, B):
            R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
def ok(res): return res.get("status") == "optimal" and res.get("time", 99) <= 60.0
def t(res): return res["time"] if ok(res) else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
def row(keys, name):
    keys = [k for k in keys if A in R[k] and B in R[k]]
    sa, sb = sgm([t(R[k][A]) for k in keys]), sgm([t(R[k][B]) for k in keys])
    na, nb = sum(ok(R[k][A]) for k in keys), sum(ok(R[k][B]) for k in keys)
    print(f"{name:14s} n={len(keys):3d}  {A}: {sa:6.2f}s ({na})  {B}: {sb:6.2f}s ({nb})  比 {sb / sa - 1:+.1%}")
    return keys
reps = sorted({k[1] for k in R})
for rep in reps:
    if rep == 102:
        continue
    row([(p, rep) for p in PROBS], f"58問 rep {rep}")
row([(p, r) for p in PROBS for r in reps if r != 102], "58問 全回")
mk = row([k for k in R if k[1] == 102], "Mittelmann")
mk2 = [k for k in mk if ok(R[k][A]) or ok(R[k][B])]
row(mk2, "Mitt 片方以上解")
print("\n片方だけ解けた回:")
for k in sorted(R):
    d = R[k]
    if A in d and B in d and ok(d[A]) != ok(d[B]):
        f = lambda r: f"{r['time']:.1f}" if ok(r) else f"TL{'(%.1fs)' % r['time'] if r.get('status') == 'optimal' else ''}"
        print(f"  {k[0]:28s} r{k[1]:3d}  {A} {f(d[A]):>12s}  {B} {f(d[B]):>12s}")
print("\n60 秒超え・異常・食い違い:")
for k, d in sorted(R.items()):
    for c, v in d.items():
        if v.get("time", 0) > 61 or v.get("status") not in ("optimal", "time_limit"):
            print("  ", k, c, v.get("status"), round(v.get("time", 0), 1), str(v.get("error", ""))[:80])
    if all(c in d and d[c].get("status") == "optimal" for c in (A, B)):
        a, b = d[A]["objective"], d[B]["objective"]
        if abs(a - b) > 1e-6 * max(1, abs(a)):
            print("   食い違い", k, a, b)
