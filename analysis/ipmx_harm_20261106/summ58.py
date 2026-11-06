"""bench58.jsonl の集計 (def と B= の設定、既定 ipmxm)。60 秒を超えて返った回は時間切れとして数える。SGM はずらし 10 秒。"""
import json, math, os, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0] / "cbcimpl_20261029"))
from probs import PROBS, SETS  # noqa: E402
A, B = "def", os.environ.get("B", "ipmxm")
R = defaultdict(dict)
for l in open(here / "bench58.jsonl"):
    r = json.loads(l)
    R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
def ok(res): return res.get("status") == "optimal" and res.get("time", 99) <= 60.0
def t(res): return res["time"] if ok(res) else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
def row(keys, name):
    keys = [k for k in keys if A in R[k] and B in R[k]]
    if not keys:
        return
    sa, sb = sgm([t(R[k][A]) for k in keys]), sgm([t(R[k][B]) for k in keys])
    na, nb = sum(ok(R[k][A]) for k in keys), sum(ok(R[k][B]) for k in keys)
    print(f"| {name} | {len(keys)} | {sa:.2f} ({na}) | {sb:.2f} ({nb}) | {sb / sa - 1:+.1%} |")
reps = sorted({k[1] for k in R})
print(f"| 組 | 回数 | {A} SGM (解けた数) | {B} SGM (解けた数) | 比 |\n|---|---|---|---|---|")
for rep in reps:
    row([(p, rep) for p in PROBS], f"58 問 rep {rep}")
row([(p, r) for p in PROBS for r in reps], "58 問 全回")
for s, ps in SETS.items():
    row([(p, r) for p in ps for r in reps], f"{s} 全回")
print("\n時間の差が 2 秒以上かつ 20% 以上の回 (または片方だけ解けた回):")
for k in sorted(R):
    d = R[k]
    if A in d and B in d:
        ta, tb = t(d[A]), t(d[B])
        if ok(d[A]) != ok(d[B]) or (abs(ta - tb) >= 2 and abs(ta - tb) >= 0.2 * min(ta, tb)):
            f = lambda r: f"{r['time']:.1f}" if ok(r) else f"TL({r.get('time', 0):.1f})"
            print(f"  {k[0]:28s} r{k[1]}  {A} {f(d[A]):>10s}  {B} {f(d[B]):>10s}")
print("\n60 秒を 1 秒以上超えた回・異常・目的値の食い違い:")
for k, d in sorted(R.items()):
    for c, v in d.items():
        if v.get("time", 0) > 61 or v.get("status") not in ("optimal", "time_limit"):
            print("  ", k, c, v.get("status"), round(v.get("time", 0), 1), str(v.get("error", ""))[:80])
    if all(c in d and d[c].get("status") == "optimal" for c in (A, B)):
        a, b = d[A]["objective"], d[B]["objective"]
        if abs(a - b) > 1e-6 * max(1, abs(a)):
            print("   食い違い", k, a, b)
