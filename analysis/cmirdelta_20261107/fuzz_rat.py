"""試作 ENOMOTO_MIP_CMIRDELTA の誤答の検査 (analysis/cmirdelta_20261107.md)。行の係数が「共通の分母 q (3・7・12 など) を持つ有理数 × 尺度」
(ときどき小さな揺らぎを足す) の小さな MIP を乱数で作り、HiGHS と最適値を突き合わせる。一般整数列・0-1 列・連続列 (負の係数で行に入る) を持つ。
使い方: ENOMOTO_MIP_CMIRDELTA=1 python fuzz_rat.py 問題数 [開始seed]"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed)
    ni, nb, nc = g.randint(3, 15), g.randint(0, 8), g.randint(0, 4)
    n = ni + nb + nc
    lo, up, c, it = [], [], [], []
    for j in range(ni):
        l = float(g.choice([0, 0, -3])); lo.append(l); up.append(l + g.randint(1, 12)); it.append(True)
    for j in range(nb):
        lo.append(0.0); up.append(1.0); it.append(True)
    for j in range(nc):
        lo.append(0.0); up.append(float(g.randint(1, 30))); it.append(False)
    for j in range(n):
        c.append(-round(g.uniform(0.5, 10), 2) if j < ni + nb else round(g.uniform(0, 3), 2))  # 最小化: 整数列は大きくしたい
    x0 = [float(g.randint(int(lo[j]), int(up[j]))) if it[j] else round(g.uniform(0, up[j]), 3) for j in range(n)]
    rows, rlo, rup = [], [], []
    for _ in range(g.randint(2, 10)):
        q = g.choice([3, 7, 12, 5, 9, 1000])
        s = g.choice([1.0, 0.37, 13.0, 2.5])
        r = {}
        for _ in range(g.randint(2, min(n, 9))):
            j = g.randrange(n)
            if it[j]:
                v = g.randint(1, 3 * q) / q * s
                if g.random() < 0.15: v *= 1 + g.uniform(-1e-3, 1e-3)
            else:
                v = -round(g.uniform(0.2, 3), 2)
            r[j] = round(v, 12)
        r = list(r.items())
        act = sum(v * x0[j] for j, v in r)
        slack = g.choice([0.0, 0.3, 1.0, 2.7]) - (0.5 if seed % 3 == 0 else 0.0)
        if g.random() < 0.8:
            rows.append(r); rlo.append(-inf); rup.append(act + slack)
        else:
            rows.append(r); rlo.append(act - slack); rup.append(act + slack + 1.0)
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); stats = {}
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    stats[hmap] = stats.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "highs statuses", stats, "time %.1f" % (time.time() - t0))
