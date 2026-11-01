"""折れ線のスコアの計測 (ab6_s*.jsonl、3 回) の集計: 既定 2 本 (base・base2) を合わせた SGM と比べる。一般整数の列が 10 本以上ある
16 問 (GEN) も別に出す。既定 2 本とも解けずに解けた / 2 本とも解けて解けなかった (問題, 回) の数、最適値の食い違いも出す"""
import json, math, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from probs import SETS  # noqa: E402
names = ["base", "base2", "sbt_abpr", "sbt_abpr_ps", "sbt_full_ps"]
GEN = ["neos-3083819-nubu", "roll3000", "mzzv42z", "neos-1456979", "mik-250-20-75-4", "gt2", "qnet1", "timtab1", "30n20b8", "gen-ip002",
       "nu25-pr12", "gen-ip054", "blend2", "noswot", "rout", "flugpl"]
R = {}
for fn in sorted(HERE.glob("ab6_s*.jsonl")):
    for l in open(fn, encoding="utf-8"):
        r = json.loads(l)
        R.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r["result"]
full = {k: v for k, v in R.items() if all(c in v for c in names)}
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def ok(x): return x.get("status") == "optimal" and x.get("time", 99) < 60
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
reps = sorted({k[1] for k in full})
print(f"そろった (問題, 回): {len(full)}")
sets = list(SETS.items()) + [("all58", sorted({p for ps in SETS.values() for p in ps})), ("一般整数16問", GEN)]
for sn, ps in sets:
    keys = [k for k in full if k[0] in ps]
    b = sgm([t(full[k][c]) for k in keys for c in ("base", "base2")])
    print(f"== {sn}: {len(keys)} (問題, 回)、既定 2 本の SGM {b:.2f}")
    for c in names:
        xs = [t(full[k][c]) for k in keys]
        per = []
        for rep in reps:
            kk = [k for k in keys if k[1] == rep]
            per.append(sgm([t(full[k][c]) for k in kk]) / sgm([t(full[k][d]) for k in kk for d in ("base", "base2")]) - 1)
        print(f"  {c:12s} {sgm(xs):6.2f} {sgm(xs) / b - 1:+6.1%}  (" + ", ".join(f"回{r} {v:+6.1%}" for r, v in zip(reps, per)) + f")  解けた {sum(x < 60 for x in xs)}/{len(xs)}")
print("\n既定 2 本とも解けず→解けた / 2 本とも解けて→解けず:")
for c in names[2:]:
    g = [f"{p}#{r}" for (p, r), d in sorted(full.items()) if ok(d[c]) and not (ok(d["base"]) or ok(d["base2"]))]
    l = [f"{p}#{r}" for (p, r), d in sorted(full.items()) if not ok(d[c]) and ok(d["base"]) and ok(d["base2"])]
    print(f"  {c:12s} +{len(g)} {g}\n  {'':12s} -{len(l)} {l}")
print("\n一般整数16問の問題ごと (秒、回ごとに " + " ".join(names) + "):")
for p in GEN:
    ks = sorted(k for k in full if k[0] == p)
    print(f"  {p:20s} " + " | ".join(" ".join(f"{t(full[k][c]):4.0f}" for c in names) for k in ks))
solu = {}
for l in open(HERE.parents[1] / "scripts" / "miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        solu[s[1]] = float(s[2])
bad = [(p, rep, c, v.get("objective")) for (p, rep), d in R.items() for c, v in d.items()
       if (v.get("status") == "optimal" and p in solu and abs(v["objective"] - solu[p]) > 1e-6 * max(1, abs(solu[p])) + 1e-6) or v.get("status") in ("crash", "error")]
print("\n最適値の食い違い・異常終了:", bad)
