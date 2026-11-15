"""容量の定義 `z = Σ c y` (z は一般整数、y は 0-1、c は整数) と容量の行 `Σ d x <= z` を持つ小さな MIP を作り、HiGHS と最適値を
突き合わせる (analysis/presolvegap_20261113.md、`ENOMOTO_MIP_TREE2_INTAGG=d` が働く形)。scripts/mip_fuzz_vs_highs.py の highs・ours を使う。
一部は z を別の行にも出す・係数を整数でなくする・z の上限を締める (implied free でない) など、消してはいけない形も混ぜる。
python fuzz_cap.py 問題数 [開始seed]   (ENOMOTO_MIP_REL_GAP=1e-9 を付ける)"""
import random, sys, time
from pathlib import Path
sys.argv_saved = list(sys.argv)
sys.argv = [sys.argv[0], "0"]
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import mip_fuzz_vs_highs as base  # noqa: E402
sys.argv = sys.argv_saved
inf = float("inf")
def gen(seed):
    g = random.Random(seed)
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    def var(l, u, cost, integ):
        lo.append(l); up.append(u); c.append(cost); it.append(integ); return len(lo) - 1
    nf = g.randint(1, 4)
    for f in range(nf):
        ys = [var(0.0, 1.0, float(g.randint(1, 30)), True) for _ in range(g.randint(1, 4))]
        cs = [g.choice([1, 2, 3, 5, 8, 10]) for _ in ys]
        zub = float(sum(cs)) if g.random() < 0.7 else float(g.randint(1, max(1, sum(cs))))
        z = var(0.0, zub, 0.0 if g.random() < 0.8 else float(g.randint(-3, 3)), g.random() < 0.9)
        r = [(z, 1.0)] + [(y, -float(cc) if g.random() < 0.9 else -cc - 0.5) for y, cc in zip(ys, cs)]
        rows.append(r); rlo.append(0.0); rup.append(0.0)
        xs = [var(0.0, 1.0 if g.random() < 0.7 else float(g.randint(1, 3)), -float(g.randint(1, 20)), g.random() < 0.85) for _ in range(g.randint(1, 5))]
        rows.append([(x, float(g.randint(1, 6))) for x in xs] + [(z, -1.0)]); rlo.append(-inf); rup.append(0.0)
        if g.random() < 0.3:
            rows.append([(z, 1.0), (xs[0], 1.0)]); rlo.append(-inf); rup.append(float(g.randint(1, 10)))
    n = len(lo)
    for _ in range(g.randint(0, 3)):
        r = {}
        for _ in range(2 + g.randrange(3)):
            j = g.randrange(n); r[j] = float(g.randint(-3, 3))
        r = [(j, v) for j, v in r.items() if v]
        if r:
            rows.append(r); rlo.append(-inf); rup.append(float(g.randint(0, 8)))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = base.highs(lp); os_, oo, x = base.ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10:
            print("seed", seed, "highs", hs, ho, "ours", os_, oo)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
