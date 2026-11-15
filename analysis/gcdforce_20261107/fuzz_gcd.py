"""係数が公約数を持つ整数列だけの行 (右辺は g の倍数でない) と、ずらした上下限を持つ小さな MIP を作り、HiGHS と最適値を突き合わせる
(GCDFORCE の判定に当たる形: 丸めた右辺が活動値の端と一致する・幅に g の倍数がない、など)。
使い方: python fuzz_gcd.py 問題数 [開始seed] (ENOMOTO_MIP_GCDFORCE=1 などを付けて)"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
spec_path = WT / "scripts/mip_fuzz_vs_highs.py"
src = open(spec_path).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, str(spec_path), "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); n = g.randint(3, 12)
    lo = [float(g.randint(-3, 2)) for _ in range(n)]
    up = [l + g.choice([1, 1, 2, 3, 6]) for l in lo]
    c = [round((g.random() - 0.5) * 20, 2) for _ in range(n)]
    it = [True] * n
    if g.random() < 0.3:  # 連続列を少し (gcd の行には入れない)
        k = g.randrange(n); it[k] = False
    rows = []; rlo = []; rup = []
    ints = [j for j in range(n) if it[j]]
    for _ in range(g.randint(1, 5)):
        gg = g.choice([2, 2, 3, 4, 5])
        js = g.sample(ints, g.randint(1, min(4, len(ints))))
        row = [(j, float(gg * g.choice([1, 1, 2, -1]))) for j in js]
        mn = sum(a * (lo[j] if a > 0 else up[j]) for j, a in row); mx = sum(a * (up[j] if a > 0 else lo[j]) for j, a in row)
        t = g.choice([0, 1, 2, 3, 3, 3])
        off = g.randint(1, gg - 1) if gg > 1 else 0
        if t == 0:  # <= mn + (g の倍数でない余り): 丸めると強制
            rows.append(row); rlo.append(-inf); rup.append(mn + off)
        elif t == 1:  # >= mx - 余り: 丸めると強制
            rows.append(row); rlo.append(mx - off); rup.append(inf)
        elif t == 2:  # 幅の中に g の倍数が 0 か 1 つ
            v = g.randint(int(mn), int(mx)); rows.append(row); rlo.append(v + 0.5 * g.random()); rup.append(v + g.randint(0, gg - 1) + 0.5)
        else:
            v = g.uniform(mn, mx); rows.append(row); rlo.append(-inf); rup.append(round(v, 1))
    if len(rows) and g.random() < 0.5 and not all(it):  # 連続列を含む行
        k = [j for j in range(n) if not it[j]][0]; js = g.sample(range(n), min(3, n))
        row = [(j, float(g.randint(1, 4))) for j in set(js + [k])]; rows.append(row); rlo.append(-inf); rup.append(float(g.randint(0, 10)))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); cnt = {}
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    cnt[hmap] = cnt.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0), cnt)
