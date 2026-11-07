"""HNFPRE 向けの乱数の問題で HiGHS と最適値を突き合わせる (python fuzz_hnf.py 問題数 [開始seed])。
整数列 (0-1 と幅のある一般整数) と、係数 2〜15 (ときどき 1/2・1/3 の有理数) の等式、固定した連続列を含む等式、
右辺を 1 ずらして格子に入らなくした等式などを作る。ENOMOTO_MIP_HNFPRE を付けて動かす。"""
import random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import types  # noqa: E402
# scripts/mip_fuzz_vs_highs.py の highs・ours だけ使う (あのファイルは読み込むと計測を始めるので、計測の前までを実行する)
_src = (Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py").read_text().split("\nN=int(")[0]
F = types.ModuleType("F"); exec(_src, F.__dict__)
inf = float('inf')

def gen(seed):
    g = random.Random(seed)
    # 奇数の seed: 列が多く行が長い (前処理で消えにくい) 一般整数の問題
    hard = seed % 2 == 1
    n = g.randint(8, 20) if hard else g.randint(3, 14); m = g.randint(2, 7) if hard else g.randint(1, 6)
    lo, up, c, it = [], [], [], []
    for j in range(n):
        t = g.randrange(6)
        if hard and t <= 1 and g.random() < 0.7: t = 2
        if t <= 1: l, u = 0.0, 1.0
        elif t <= 3: l = float(g.randrange(-5, 3)); u = l + g.randrange(1, 40)
        elif t == 4: l, u = 0.0, inf
        else: l = u = float(g.randrange(-3, 4))  # 固定
        lo.append(l); up.append(u); c.append(round((g.random() - 0.5) * 20, 1)); it.append(t != 5 or g.random() < 0.5)
    x0 = []
    for j in range(n):
        v = lo[j] + ((up[j] - lo[j]) * g.random() if up[j] < inf else 10 * g.random())
        if it[j]: v = min(max(round(v), lo[j]), up[j])
        x0.append(v)
    rows, rlo, rup = [], [], []
    for i in range(m):
        r = {}
        for _ in range((3 + g.randrange(5)) if hard else (2 + g.randrange(4))):
            j = g.randrange(n)
            v = float(g.choice([2, 3, 4, 5, 6, 7, 9, 10, 12, 15, 1, -2, -3, -6]))
            if g.random() < 0.15: v = v / g.choice([2, 3])
            r[j] = v
        r = list(r.items())
        act = sum(v * x0[j] for j, v in r)
        k = g.randrange(5)
        if k == 0: act += 1  # 格子に入らないかもしれない
        if k <= 3: rlo.append(act); rup.append(act)
        else: rlo.append(act - 3 * g.random()); rup.append(act + 3 * g.random())
        rows.append(r)
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-inf); rup.append(sum(abs(v) for v in x0) + 30)
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
