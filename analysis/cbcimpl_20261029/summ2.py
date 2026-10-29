"""1 段目 (22 設定) の集計: 既定は base と base2 の 2 本を合わせた SGM (揺れを均す) と比べる。
使い方: python summ2.py [<設定名> ...]  (既定は cfgs.py の 1 段目の設定)"""
import json, math, sys, glob
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from probs import SETS  # noqa: E402
from cfgs import CFGS  # noqa: E402
names = sys.argv[1:] or [c for c in CFGS if c not in ("sb_inh", "sbt_a", "sbt_ab")]
R = {}
for fn in [HERE / "ab_r1.jsonl"] + sorted(HERE.glob("ab_s[0-9].jsonl")):
    for l in open(fn, encoding="utf-8"):
        r = json.loads(l)
        R.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r["result"]
full = {k: v for k, v in R.items() if all(c in v for c in names)}
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
print(f"そろった (問題, 回): {len(full)}")
for sn, ps in SETS.items():
    keys = [k for k in full if k[0] in ps]
    base = sgm([t(full[k][c]) for k in keys for c in ("base", "base2")])
    print(f"== {sn}: {len(keys)} (問題, 回)、既定 2 本の SGM {base:.2f}")
    for c in names:
        xs = [t(full[k][c]) for k in keys]
        s = sgm(xs)
        print(f"  {c:13s} {s:6.2f} {s / base - 1:+6.1%}  解けた {sum(x < 60 for x in xs)}/{len(xs)}")
