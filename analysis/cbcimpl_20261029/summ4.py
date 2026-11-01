"""強分岐の境界の引き継ぎの計測 (ab3_s0.jsonl・ab4_s*.jsonl) の集計: 6 設定がそろった (問題, 回) で、base と比べた SGM、
問題ごとの大きな差、最適値の食い違い。使い方: python summ4.py"""
import json, math, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from probs import SETS  # noqa: E402
names = ["base", "sb_tighten", "sb_inh", "sbt_a", "sbt_abp", "sbt_abpr"]
R = {}
for fn in [HERE / "ab3_s0.jsonl"] + sorted(HERE.glob("ab4_s*.jsonl")):
    for l in open(fn, encoding="utf-8"):
        r = json.loads(l)
        R.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r["result"]
full = {k: v for k, v in R.items() if all(c in v for c in names)}
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
print(f"そろった (問題, 回): {len(full)}")
for sn, ps in list(SETS.items()) + [("all58", sorted({p for ps in SETS.values() for p in ps}))]:
    keys = [k for k in full if k[0] in ps]
    b = sgm([t(full[k]["base"]) for k in keys])
    print(f"== {sn}: {len(keys)} (問題, 回)")
    for c in names:
        xs = [t(full[k][c]) for k in keys]
        r = [sgm([t(full[k][c]) for k in keys if k[1] == rep]) / sgm([t(full[k]["base"]) for k in keys if k[1] == rep]) - 1 for rep in (0, 1)]
        print(f"  {c:11s} {sgm(xs):6.2f} {sgm(xs) / b - 1:+6.1%}  (回0 {r[0]:+6.1%}, 回1 {r[1]:+6.1%})  解けた {sum(x < 60 for x in xs)}/{len(xs)}")
print("\n問題ごと (base と 20% 以上かつ 3 秒以上違うもの):")
for (p, rep), d in sorted(full.items()):
    b = t(d["base"])
    diffs = [f"{c}={t(d[c]):.1f}" for c in names[1:] if abs(t(d[c]) - b) > max(3.0, 0.2 * b)]
    if diffs:
        print(f"  {p:22s} rep{rep} base={b:.1f}  " + " ".join(diffs))
solu = {}
for l in open(HERE.parents[1] / "scripts" / "miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        solu[s[1]] = float(s[2])
bad = [(p, rep, c, v.get("objective")) for (p, rep), d in R.items() for c, v in d.items()
       if (v.get("status") == "optimal" and p in solu and abs(v["objective"] - solu[p]) > 1e-6 * max(1, abs(solu[p])) + 1e-6) or v.get("status") in ("crash", "error")]
print("\n最適値の食い違い・異常終了:", bad)
