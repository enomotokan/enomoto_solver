"""SNFMODK 向けの乱数の小さな MIP (整数列だけ、係数が 4・6・9・12 などの倍数の等式・不等式で、群に Z/4・Z/6 などが出やすい形) を
HiGHS と突き合わせる。使い方: python fuzz_snf.py 問題数 [開始seed]  (試作は ENOMOTO_MIP_SNFMODK=c などを付けて)"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[3]
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); n = g.randint(3, 14); m = g.randint(2, 9)
    lo = []; up = []; c = []; it = []
    for j in range(n):
        l = -float(g.randrange(3)); u = l + 1 + g.randrange(12)
        lo.append(l); up.append(u); c.append(round((g.random() - 0.5) * 20, 2)); it.append(g.random() < 0.9)
    rows = []; rlo = []; rup = []
    for _ in range(m):
        k = g.choice([2, 3, 4, 6, 6, 9, 12, 10, 15])
        js = g.sample(range(n), g.randint(1, min(n, 5)))
        r = [(j, float(k * g.choice([-2, -1, 1, 1, 2]))) for j in js]
        # 係数 1 の列を足すこともある (群の消去で消える)
        if g.random() < 0.4:
            r.append((g.choice([j for j in range(n) if j not in js] or js[:1]), float(g.choice([-1, 1, 2, 3]))))
        d = {}
        for j, v in r:
            d[j] = d.get(j, 0.0) + v
        r = [(j, v) for j, v in d.items() if v != 0]
        if not r:
            continue
        b = float(g.randint(-20, 30))
        t = g.randrange(3)
        if t == 0: rlo.append(b); rup.append(b)
        elif t == 1: rlo.append(-inf); rup.append(b)
        else: rlo.append(b); rup.append(inf)
        rows.append(r)
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); nopt = 0
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    nopt += hmap == "optimal"
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "optimal", nopt, "time %.1f" % (time.time() - t0))
