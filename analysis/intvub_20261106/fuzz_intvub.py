"""一般整数列 w と 0-1 列 z の変数上下限 (w <= u z、w >= l z、係数の倍率・符号・等式・整数でない u を混ぜる) と、w・z・連続列の
データの行を持つ小さな MIP を乱数で作り、HiGHS と最適値を突き合わせる (試作 ENOMOTO_MIP_INTVUB の誤答の検査、analysis/intvub_20261106.md)。
HiGHS と ENOMOTO の呼び出しは scripts/mip_fuzz_vs_highs.py のもの (ENOMOTO は mip_rel_gap=1e-9)。
使い方: ENOMOTO_MIP_INTVUB=1 python fuzz_intvub.py 問題数 [開始seed]  (ENOMOTO_MIP_INTVUB_LOG=1 なら集めた数が標準エラーに出る)"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed)
    nw, nz, nc = g.randint(3, 12), g.randint(2, 8), g.randint(0, 4)
    n = nw + nz + nc
    lo, up, c, it = [], [], [], []
    for j in range(nw):
        l = -float(g.choice([0, 0, 0, 2, 5, 10])); u = float(g.randint(2, 12))
        lo.append(l); up.append(u); it.append(True); c.append(round((g.random() - 0.5) * 6, 2))
    for j in range(nz):
        lo.append(0.0); up.append(1.0); it.append(True); c.append(round(g.random() * 5, 2))
    for j in range(nc):
        lo.append(0.0); up.append(float(g.randint(1, 20))); it.append(False); c.append(round((g.random() - 0.5) * 4, 2))
    rows, rlo, rup = [], [], []
    for w in range(nw):
        z = nw + g.randrange(nz)
        s = g.choice([1.0, 1.0, 2.0, -1.0, 3.0])
        uu = g.choice([float(g.randint(1, int(up[w]))), round(g.uniform(1, up[w]), 2)])
        if g.random() < 0.8:  # w <= uu z
            rows.append([(w, s), (z, -s * uu)]); (rlo.append(-inf), rup.append(0.0)) if s > 0 else (rlo.append(0.0), rup.append(inf))
        if lo[w] < 0 and g.random() < 0.7:  # w >= lo z (符号が自由な列)
            ll = g.choice([lo[w], float(g.randint(int(lo[w]), -1))])
            rows.append([(w, s), (z, -s * ll)]); (rlo.append(0.0), rup.append(inf)) if s > 0 else (rlo.append(-inf), rup.append(0.0))
        if g.random() < 0.1:  # 等式 w + k z = r
            k = float(g.randint(1, 3)); r = float(g.randint(int(lo[w]), int(up[w])))
            rows.append([(w, 1.0), (z, k)]); rlo.append(r); rup.append(r)
    for _ in range(g.randint(2, 10)):  # データの行
        r = {}
        for _ in range(g.randint(2, 7)):
            j = g.randrange(n); v = float(g.choice([1, -1, 1, -1, 2, -3, 0.5]))
            r[j] = v
        r = list(r.items())
        # 0 (全部 0) が満たさない行も作る (w を 0 にできない → z を立てる)
        b = g.choice([-1.0, -2.0, 1.0, 3.0, -0.5])
        if g.random() < 0.5: rows.append(r); rlo.append(-inf); rup.append(b)
        else: rows.append(r); rlo.append(-b); rup.append(inf)
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
