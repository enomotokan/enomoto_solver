"""試作 ENOMOTO_MIP_REDSPLIT の誤答の検査 (analysis/redsplit_20261107.md): 単体表の行に連続の非基底列が多く入る小さな MIP
(一般整数列 + 連続列、小数の係数の密な行、等式を混ぜる) を乱数で作り、HiGHS と最適値を突き合わせる。
HiGHS と ENOMOTO の呼び出しは scripts/mip_fuzz_vs_highs.py のもの (ENOMOTO は mip_rel_gap=1e-9)。
使い方: ENOMOTO_MIP_REDSPLIT=ab python fuzz_rs.py 問題数 [開始seed]"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed)
    ni, nc = g.randint(4, 15), g.randint(3, 15)
    n = ni + nc
    lo, up, c, it, x0 = [], [], [], [], []
    for j in range(n):
        integ = j < ni
        l = -float(g.choice([0, 0, 3])) ; u = l + float(g.choice([1, 5, 10, 30])) if g.random() < 0.85 else inf
        lo.append(l); up.append(u); it.append(integ); c.append(round((g.random() - 0.5) * 10, 3))
        v = l + (min(u, l + 10) - l) * g.random()
        if integ:
            v = float(round(v))
        x0.append(min(max(v, l), u))
    rows, rlo, rup = [], [], []
    for _ in range(g.randint(3, 12)):
        k = g.randint(2, min(n, 8))
        cols = g.sample(range(n), k)
        r = [(j, round((g.random() - 0.5) * 20, g.choice([0, 1, 3]))) for j in cols]
        r = [(j, v) for j, v in r if v != 0]
        if not r:
            continue
        act = sum(v * x0[j] for j, v in r)
        t = g.randrange(4)
        if t == 0: rlo.append(-inf); rup.append(act + g.random() * 3)
        elif t == 1: rlo.append(act - g.random() * 3); rup.append(inf)
        elif t == 2: rlo.append(act - g.random() * 2); rup.append(act + g.random() * 2)
        else: rlo.append(act); rup.append(act)
        rows.append(r)
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-sum(abs(v) for v in x0) - 30); rup.append(sum(abs(v) for v in x0) + 30)
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); cnt = {}
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    cnt[hmap] = cnt.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, cnt, "time %.1f" % (time.time() - t0))
