"""プロービング・LP による固定が働く形の乱数の MIP (0-1 列 30〜80 本: 集合詰め込み・含意 x_a <= x_b・補の等式 x_a + x_b = 1・
ナップサック・連続列の変数上限 y <= U x) を作り、HiGHS と最適値を突き合わせる (analysis/prepar_20261107.md)。
使い方: python fuzz_probe.py 問題数 [開始seed]  (試作は ENOMOTO_MIP_PREPAR=ple を付けて回す)"""
import random, sys, time
from pathlib import Path
import importlib.util
spec = importlib.util.spec_from_file_location("fz", str(Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"))
src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec.origin, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); nb = g.randint(30, 80); ny = g.randint(0, 10)
    n = nb + ny
    lo = [0.0] * n; up = [1.0] * nb + [float(g.randint(1, 20)) for _ in range(ny)]
    c = [round((g.random() - 0.7) * 10, 2) for _ in range(nb)] + [-round(g.random() * 5, 2) for _ in range(ny)]
    it = [True] * nb + [False] * ny
    rows = []; rlo = []; rup = []
    for _ in range(g.randint(nb // 4, nb // 2)):  # 集合詰め込み
        xs = g.sample(range(nb), g.randint(2, 6)); rows.append([(j, 1.0) for j in xs]); rlo.append(-inf); rup.append(1.0)
    for _ in range(g.randint(nb // 8, nb // 3)):  # 含意 x_a <= x_b
        a, b = g.sample(range(nb), 2); rows.append([(a, 1.0), (b, -1.0)]); rlo.append(-inf); rup.append(0.0)
    for _ in range(g.randint(0, 3)):  # 補 x_a + x_b = 1
        a, b = g.sample(range(nb), 2); rows.append([(a, 1.0), (b, 1.0)]); rlo.append(1.0); rup.append(1.0)
    for _ in range(g.randint(1, 4)):  # ナップサック
        xs = g.sample(range(nb), g.randint(4, min(20, nb))); w = [float(g.randint(1, 30)) for _ in xs]
        rows.append(list(zip(xs, w))); rlo.append(-inf); rup.append(float(int(sum(w) * g.uniform(0.2, 0.6))))
    for k in range(ny):  # 変数上限 y <= U x と、y の和の下限
        x = g.randrange(nb); rows.append([(nb + k, 1.0), (x, -up[nb + k])]); rlo.append(-inf); rup.append(0.0)
    if ny:
        rows.append([(nb + k, 1.0) for k in range(ny)]); rlo.append(float(g.randint(0, 5))); rup.append(inf)
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
