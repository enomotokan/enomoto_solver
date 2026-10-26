"""幅 1e-9 の連続列とクリーク型の行 (`Σx + Σy <= 1 - k 1e-9`) を持つ小さな MIP を作り、HiGHS と最適値を突き合わせる。
使い方: python fuzz_tiny.py 問題数 [開始seed]  (ENOMOTO_MIP_COEF_TIGHTEN_TINY=1 で以前の係数の強化)"""
import random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import importlib.util
spec = importlib.util.spec_from_file_location("fz", str(Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"))
src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec.origin, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); nb = g.randint(3, 10); ny = g.randint(2, 8)
    lo = [0.0] * (nb + ny); up = [1.0] * nb + [1e-9 * g.choice([1, 1, 2]) for _ in range(ny)]
    c = [-round(g.random() * 10 + 1, 2) for _ in range(nb)] + [-round(g.random() * 300, 2) for _ in range(ny)]
    it = [True] * nb + [False] * ny
    rows = []; rlo = []; rup = []
    for _ in range(g.randint(2, 8)):
        xs = g.sample(range(nb), g.randint(1, min(3, nb))); ys = g.sample(range(nb, nb + ny), g.randint(1, min(4, ny)))
        rows.append([(j, 1.0) for j in xs + ys]); rlo.append(-inf); rup.append(1.0 - 1e-9 * g.randint(0, 2))
    for _ in range(g.randint(0, 3)):  # x_a - x_b + y = 0 の形
        a, b = g.sample(range(nb), 2); y = g.randrange(nb, nb + ny)
        rows.append([(a, -1.0), (b, 1.0), (y, 1.0)]); rlo.append(0.0); rup.append(0.0)
    for _ in range(g.randint(1, 3)):  # 普通のナップサック
        xs = g.sample(range(nb), g.randint(2, nb)); rows.append([(j, float(g.randint(1, 9))) for j in xs]); rlo.append(-inf); rup.append(float(g.randint(5, 20)))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
