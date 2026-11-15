"""試作 NODELP2 の誤答の検査: 木が大きくなる型 (多制約ナップサック・集合被覆・一般整数の割当て) の小さな MIP を作り、
HiGHS と最適値を突き合わせる (scripts/mip_fuzz_vs_highs.py の highs/ours を使う)。
使い方: ENOMOTO_MIP_NODELP2=pbx ENOMOTO_T_MIP_NODELP2_WARM=1 ENOMOTO_T_MIP_NODELP2_EXPLORE=2 ENOMOTO_MIP_THREADS=4 python fuzz_tree.py 問題数 [開始seed]"""
import random, sys, time, importlib.util
from pathlib import Path
sp = Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"
src = open(sp).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, str(sp), "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); kind = seed % 3
    if kind == 0:  # 多制約ナップサック (最大化を最小化に)
        n = g.randint(40, 70); m = g.randint(3, 6)
        base = [g.randint(5, 60) for _ in range(n)]
        c = [-float(base[j] + 10) for j in range(n)]  # 費用と重さが相関 (木が大きくなる)
        rows = []; rlo = []; rup = []
        for _ in range(m):
            w = [max(1, base[j] + g.randint(-3, 3)) for j in range(n)]
            rows.append([(j, float(w[j])) for j in range(n)]); rlo.append(-inf); rup.append(float(sum(w) // 2))
        return [0.0] * n, [1.0] * n, c, [True] * n, rows, rlo, rup
    if kind == 1:  # 集合被覆
        n = g.randint(80, 140); m = g.randint(50, 90)
        c = [float(g.randint(1, 20)) for _ in range(n)]
        rows = []
        for _ in range(m):
            k = g.randint(2, 6); rows.append([(j, 1.0) for j in g.sample(range(n), k)])
        return [0.0] * n, [1.0] * n, c, [True] * n, rows, [1.0] * m, [inf] * m
    # 一般整数 + 連続 (容量つき割当て)
    n = g.randint(15, 30); m = g.randint(4, 8)
    c = [round(g.uniform(-10, 10), 2) for _ in range(n)]
    up = [float(g.randint(1, 5)) for _ in range(n)]
    it = [g.random() < 0.7 for _ in range(n)]
    rows = []; rlo = []; rup = []
    for _ in range(m):
        cols = g.sample(range(n), g.randint(4, n))
        rows.append([(j, float(g.randint(-5, 9))) for j in cols]); rlo.append(float(g.randint(-10, 0))); rup.append(float(g.randint(5, 25)))
    return [0.0] * n, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
