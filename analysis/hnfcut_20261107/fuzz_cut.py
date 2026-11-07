"""HNFCUT 向けの乱数の問題で HiGHS と最適値を突き合わせる (python fuzz_cut.py 問題数 [開始seed])。2026-11-07。
一般整数列 x に、3 列の等式 x_j - g w_j - g v_j = r (w・v は他の行にも出る整数列) で合同式 x_j ≡ r (mod g) を持たせ、
x の不等式 (係数 -9〜9) と目的関数で LP の解を分数・合同式に合わない整数にする (カットが効く形)。ときどき連続列も混ぜる。
ENOMOTO_MIP_HNFCUT を付けて動かす。ENOMOTO_MIP_LOG=1 なら標準エラーの "MIP: root HNFCUT stats" の行で、合同式のカットが出た問題を数えられる。"""
import random, sys, time
from pathlib import Path
import types
_src = (Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py").read_text().split("\nN=int(")[0]
F = types.ModuleType("F"); exec(_src, F.__dict__)
inf = float('inf')

def gen(seed):
    g = random.Random(seed)
    nx = g.randint(4, 10)
    lo, up, c, it = [], [], [], []
    def col(l, u, cost, integ):
        lo.append(float(l)); up.append(float(u)); c.append(cost); it.append(integ); return len(lo) - 1
    xs = [col(0, g.randint(8, 40), -round(g.random() * 10, 1) - 0.1, True) for _ in range(nx)]
    rows, rlo, rup = [], [], []
    aux = []
    for j in xs:
        if g.random() < 0.7:
            gg = g.choice([2, 3, 4, 5, 6, 10]); r = g.randrange(-3, 7)
            w = col(0, g.randint(3, 15), round(g.random() - 0.5, 1), True); v = col(0, g.randint(3, 15), round(g.random() - 0.5, 1), True)
            aux += [w, v]
            rows.append([(j, 1.0), (w, -float(gg)), (v, -float(gg))]); rlo.append(float(r)); rup.append(float(r))
    if g.random() < 0.3:
        y = col(0, 5, round(g.random() * 2, 1), False); aux.append(y)
    # x (と補助の列) の不等式
    for _ in range(g.randint(2, 6)):
        r = {}
        for j in xs:
            if g.random() < 0.6:
                r[j] = float(g.randint(1, 9) * (1 if g.random() < 0.85 else -1))
        if aux and g.random() < 0.5:
            r[g.choice(aux)] = float(g.randint(-5, 5) or 1)
        if not r: continue
        r = list(r.items())
        pt = sum(v * (up[j] if g.random() < 0.3 else up[j] * g.random()) for j, v in r)
        rows.append(r); rlo.append(-inf); rup.append(float(int(pt)) + round(g.random(), 2) * (g.random() < 0.3))
    return lo, up, c, it, rows, rlo, rup

if __name__ == "__main__":
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; t0 = time.time(); st = {}
    for seed in range(start, start + N):
        lp = gen(seed)
        print("SEED", seed, file=sys.stderr, flush=True)
        hs, ho = F.highs(lp); os_, oo, x = F.ours(lp)
        hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
        st[hmap] = st.get(hmap, 0) + 1
        ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
    print("bad", bad, "of", N, st, "time %.1f" % (time.time() - t0))
