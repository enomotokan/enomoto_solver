"""確かめの計測 (ab5_s*.jsonl) の集計: 9 設定がそろった (問題, 回) で、既定 2 本 (base・base2) を合わせた SGM と比べる。
問題ごとの大きな差、最適値の食い違いも出す。使い方: python summ5.py"""
import json, math, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from probs import SETS  # noqa: E402
names = ["base", "base2", "tab_nnz", "node_maxlen", "dive_rcfix", "combo", "sbt_abpr", "sbt_abp_rc", "sbt_abp_rcp"]
R = {}
for fn in sorted(HERE.glob("ab5_s*.jsonl")):
    for l in open(fn, encoding="utf-8"):
        r = json.loads(l)
        R.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r["result"]
full = {k: v for k, v in R.items() if all(c in v for c in names)}
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
print(f"そろった (問題, 回): {len(full)}")
for sn, ps in list(SETS.items()) + [("all58", sorted({p for ps in SETS.values() for p in ps}))]:
    keys = [k for k in full if k[0] in ps]
    b = sgm([t(full[k][c]) for k in keys for c in ("base", "base2")])
    print(f"== {sn}: {len(keys)} (問題, 回)、既定 2 本の SGM {b:.2f}")
    for c in names:
        xs = [t(full[k][c]) for k in keys]
        r = []
        for rep in (0, 1):
            kk = [k for k in keys if k[1] == rep]
            r.append(sgm([t(full[k][c]) for k in kk]) / sgm([t(full[k][d]) for k in kk for d in ("base", "base2")]) - 1)
        print(f"  {c:12s} {sgm(xs):6.2f} {sgm(xs) / b - 1:+6.1%}  (回0 {r[0]:+6.1%}, 回1 {r[1]:+6.1%})  解けた {sum(x < 60 for x in xs)}/{len(xs)}")
print("\n問題ごと (既定 2 本の大きい方より 20% 以上かつ 3 秒以上遅い / 小さい方より速いもの):")
for (p, rep), d in sorted(full.items()):
    lo, hi = sorted([t(d["base"]), t(d["base2"])])
    w = [f"{c}={t(d[c]):.1f}" for c in names[2:] if t(d[c]) > max(hi * 1.2, hi + 3)]
    f = [f"{c}={t(d[c]):.1f}" for c in names[2:] if t(d[c]) < min(lo / 1.2, lo - 3)]
    if w or f:
        print(f"  {p:22s} rep{rep} base={t(d['base']):.1f}/{t(d['base2']):.1f}  遅い: {' '.join(w)}  速い: {' '.join(f)}")
solu = {}
for l in open(HERE.parents[1] / "scripts" / "miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        solu[s[1]] = float(s[2])
bad = [(p, rep, c, v.get("objective")) for (p, rep), d in R.items() for c, v in d.items()
       if (v.get("status") == "optimal" and p in solu and abs(v["objective"] - solu[p]) > 1e-6 * max(1, abs(solu[p])) + 1e-6) or v.get("status") in ("crash", "error")]
print("\n最適値の食い違い・異常終了:", bad)
