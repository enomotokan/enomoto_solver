"""既定化の確かめの集計 (analysis/drcr_20261110.md の既定化の節)。analysis/benchlib.py の report を使う。
python summ_default.py <58 問の jsonl> [Mittelmann の jsonl]"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl  # noqa: E402
rows = bl.load([sys.argv[1]])
R = bl.table(rows)
keys = [k for k in R if "def" in R[k] and "rifold" in R[k]]
try:
    noise = bl.load_noise()
except Exception:  # noqa: BLE001
    noise = None
bl.report(R, "def", "rifold", keys, noise=noise, label="58 問 60 秒")
if len(sys.argv) > 2:
    bl.TL = 600.0
    R2 = bl.table(bl.load([sys.argv[2]]))
    k2 = [k for k in R2 if "def" in R2[k] and "rifold" in R2[k]]
    t = lambda r: bl.t(r, 600.0)
    ok = lambda r: bl.ok(r, 600.0)
    import math
    sg = lambda xs: math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
    print(f"[Mittelmann 600 秒] 鍵 {len(k2)}  SGM {sg([t(R2[k]['def']) for k in k2]):.1f} → {sg([t(R2[k]['rifold']) for k in k2]):.1f}  "
          f"解けた {sum(ok(R2[k]['def']) for k in k2)} → {sum(ok(R2[k]['rifold']) for k in k2)}")
    for k in sorted(k2):
        print(f"   {k[0]:24s} 回 {k[1]}  既定 {t(R2[k]['def']):6.1f}  ri {t(R2[k]['rifold']):6.1f}")
