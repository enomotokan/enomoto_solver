"""(mushroom_cuts_20261106/fuzz_mush.py を写して、連続列の変数上下限 c <= u z・c >= l z を足したもの、analysis/modkvb_20261107.md)
試作 ENOMOTO_MIP_MODKVB の誤答の検査。一般整数列 w の変数上下限 (w <= u z、w >= l z)、変数上下限を持たない一般整数列 x0 (全てのデータの行)、
連続列 c (多くは c <= u z、ときどき c >= l z も) と、w・z・c・x0 のデータの行を持つ小さな MIP を乱数で作り、HiGHS と最適値を突き合わせる。
使い方: ENOMOTO_MIP_MODKVB=1 python fuzz_vb.py 問題数 [開始seed]"""
import math, random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed)
    nw, nz, nc = g.randint(4, 25), g.randint(3, 12), g.randint(0, 5)
    n = nw + nz + nc + 1  # 最後の列が x0
    lo, up, c, it = [], [], [], []
    for j in range(nw):
        l = -float(g.choice([0, 0, 0, 2, 5, 10])); u = float(g.randint(2, 12))
        lo.append(l); up.append(u); it.append(True); c.append(round((g.random() - 0.5) * 6, 2))
    for j in range(nz):
        lo.append(0.0); up.append(1.0); it.append(True); c.append(round(g.random() * 5, 2))
    for j in range(nc):
        lo.append(0.0); up.append(float(g.randint(1, 20))); it.append(False); c.append(round((g.random() - 0.5) * 4, 2))
    L = g.choice([3, 10, 100])
    lo.append(-float(L)); up.append(float(L)); it.append(True); c.append(round((g.random() - 0.5) * 0.2, 3))
    # 実行可能な点 x0 を先に決め、行はそれを満たすように作る (seed の偶数は実行可能、奇数は右辺を少しきつくして実行不能になりうる)
    x0 = [0.0] * n
    for j in range(nw, nw + nz): x0[j] = float(g.random() < 0.5)
    for j in range(nw + nz, n - 1): x0[j] = round(g.uniform(0, up[j]), 3)
    # 連続列の変数上下限 (x0 で成り立つように: 相手の z が 0 なら c = 0)
    cvb = []
    for j in range(nw + nz, n - 1):
        if g.random() < 0.8:
            z = nw + g.randrange(nz); uu = g.choice([up[j], float(g.randint(1, int(up[j]))), round(g.uniform(0.5, up[j]), 2)])
            if x0[z] == 0: x0[j] = 0.0
            else: x0[j] = round(g.uniform(0, min(uu, up[j])), 3)
            cvb.append((j, z, uu))
    x0[n - 1] = float(g.randint(-L, L))
    rows, rlo, rup = [], [], []
    for w in range(nw):
        z = nw + g.randrange(nz)
        s = g.choice([1.0, 1.0, 2.0, -1.0, 3.0])
        uu = g.choice([float(g.randint(1, int(up[w]))), round(g.uniform(1, up[w]), 2)])
        ll = g.choice([lo[w], float(g.randint(int(lo[w]), 0))]) if lo[w] < 0 else 0.0
        # x0 の w は [ll z, uu z] の中の整数
        a, b = math.ceil(ll * x0[z] - 1e-9), math.floor(uu * x0[z] + 1e-9)
        x0[w] = float(g.randint(max(a, int(lo[w])), min(b, int(up[w]))))
        if g.random() < 0.8:  # w <= uu z
            rows.append([(w, s), (z, -s * uu)]); (rlo.append(-inf), rup.append(0.0)) if s > 0 else (rlo.append(0.0), rup.append(inf))
        if lo[w] < 0 and g.random() < 0.7:  # w >= ll z (符号が自由な列)
            rows.append([(w, s), (z, -s * ll)]); (rlo.append(0.0), rup.append(inf)) if s > 0 else (rlo.append(-inf), rup.append(0.0))
        if g.random() < 0.1:  # 等式 w + k z = r (x0 で成り立つ値)
            k = float(g.randint(1, 3)); r = x0[w] + k * x0[z]
            rows.append([(w, 1.0), (z, k)]); rlo.append(r); rup.append(r)
    for j, z, uu in cvb:  # c <= uu z、ときどき c >= ll z (ll <= x0 の値 / z)
        s = g.choice([1.0, 2.0, -1.0])
        rows.append([(j, s), (z, -s * uu)]); (rlo.append(-inf), rup.append(0.0)) if s > 0 else (rlo.append(0.0), rup.append(inf))
        if x0[z] == 1 and g.random() < 0.3:
            ll = round(g.uniform(0, x0[j]), 2)
            rows.append([(j, 1.0), (z, -ll)]); rlo.append(0.0); rup.append(inf)
    for _ in range(g.randint(3, 15)):  # データの行
        r = {}
        for _ in range(g.randint(2, 8)):
            j = g.randrange(nw) if g.random() < 0.5 else g.randrange(n - 1); r[j] = float(g.choice([1, -1, 1, -1, 2, -3, 0.5]))
        r[n - 1] = float(g.choice([1, -1, 1, -1, 2, -2]))
        r = list(r.items())
        act = sum(v * x0[j] for j, v in r)
        slack = g.choice([0.0, 0.0, 0.5, 1.0, 2.0]) - (0.7 if seed % 2 else 0.0)
        if g.random() < 0.5: rows.append(r); rlo.append(-inf); rup.append(act + slack)
        else: rows.append(r); rlo.append(act - slack); rup.append(inf)
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
