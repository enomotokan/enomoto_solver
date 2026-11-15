"""ENOMOTO_MIP_HNFPRE_LAT 向けの乱数の問題で HiGHS と最適値を突き合わせる (python fuzz_lat.py 問題数 [開始seed])。
上下限のある整数列 (0-1 と幅のある一般整数) 3〜14 本、連続列 0〜3 本 (等式のスラックになることがある)、係数 0〜20 の等式 1〜3 本
(整数列 3 本以上)、不等式 0〜3 本、目的はでたらめ。書き直しの条件 (hnflat::reform) に合うように作る。"""
import random, sys, time, types
from pathlib import Path
_src = (Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py").read_text().split("\nN=int(")[0]
F = types.ModuleType("F"); exec(_src, F.__dict__)
inf = float('inf')

def gen(seed):
    g = random.Random(seed); ni = g.randint(3, 14); nc = g.randint(0, 3); n = ni + nc
    lo, up, c, it = [], [], [], []
    for j in range(ni):
        if g.random() < 0.6: l, u = 0.0, 1.0
        else: l = float(g.randrange(-4, 3)); u = l + g.randrange(1, 12)
        lo.append(l); up.append(u); it.append(True); c.append(round((g.random() - 0.5) * 20, 1))
    for j in range(nc):
        lo.append(0.0); up.append(inf if g.random() < 0.5 else float(g.randrange(1, 30))); it.append(False); c.append(round(g.random() * 10, 1))
    x0 = [g.randint(int(lo[j]), int(up[j])) if it[j] else 0.0 for j in range(n)]
    rows, rlo, rup = [], [], []
    for i in range(g.randint(1, 3)):
        js = g.sample(range(ni), g.randint(3, ni))
        r = [(j, float(g.randrange(1, 21) * g.choice([1, 1, 1, -1]))) for j in js]
        if nc and g.random() < 0.6:
            r.append((ni + g.randrange(nc), g.choice([1.0, -1.0])))
        act = sum(a * x0[j] for j, a in r if it[j])
        if g.random() < 0.3: act += g.randint(-2, 2)
        rows.append(r); rlo.append(act); rup.append(act)
    for i in range(g.randint(0, 3)):
        js = g.sample(range(n), g.randint(2, min(n, 5)))
        r = [(j, float(g.randrange(-9, 10) or 1)) for j in js]
        act = sum(a * x0[j] for j, a in r)
        rows.append(r); rlo.append(-inf); rup.append(act + g.random() * 5)
    return lo, up, c, it, rows, rlo, rup

if __name__ == "__main__":
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; t0 = time.time(); st = {}
    for seed in range(start, start + N):
        lp = gen(seed)
        hs, ho = F.highs(lp); os_, oo, x = F.ours(lp)
        hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
        st[hmap] = st.get(hmap, 0) + 1
        ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
    print("bad", bad, "of", N, st, "time %.1f" % (time.time() - t0))
