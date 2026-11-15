"""CLQACT の誤答の検査: 集合詰め込みの行 (否定の文字を含む) と big-M の行 (x <= sum M_k y_k、y はクリーク) を持つ小さな MIP を作り、
HiGHS と最適値を突き合わせる。使い方: ENOMOTO_MIP_CLQACT=1 python fuzz_clqact.py 問題数 [開始seed]
(scripts/mip_fuzz_vs_highs.py の highs()・ours() を使う。比べるのは状態と最適値 (相対 1e-6))"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[3]
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}
exec(compile(src, "mip_fuzz_vs_highs", "exec"), ns)
highs, ours, inf = ns["highs"], ns["ours"], float("inf")
def gen(seed):
    g = random.Random(seed)
    nb = g.randint(5, 14); nc = g.randint(1, 4); n = nb + nc
    lo = [0.0] * n; up = [1.0] * nb + [float(g.randint(3, 40)) for _ in range(nc)]
    it = [True] * nb + [g.random() < 0.4 for _ in range(nc)]
    c = [round((g.random() - 0.5) * 20, 2) for _ in range(n)]
    rows, rlo, rup = [], [], []
    cliques = []
    for _ in range(g.randint(2, 5)):
        S = g.sample(range(nb), g.randint(2, min(nb, 6)))
        neg = {j for j in S if g.random() < 0.2}
        # sum_{S\\neg} y + sum_{neg} (1 - y) <= 1
        rows.append([(j, -1.0 if j in neg else 1.0) for j in S]); rlo.append(-inf); rup.append(1.0 - len(neg))
        if not neg: cliques.append(S)
    for k in range(nc):
        j = nb + k
        S = g.choice(cliques) if cliques and g.random() < 0.8 else g.sample(range(nb), g.randint(2, min(nb, 4)))
        # x_j - sum M_k y_k <= 0 (big-M、M は x の上限より大きいことも小さいこともある)
        rows.append([(j, 1.0)] + [(y, -float(g.randint(2, 60))) for y in S]); rlo.append(-inf); rup.append(float(g.randint(0, 3)))
        if g.random() < 0.5:
            rows.append([(j, 1.0)] + [(y, float(g.randint(-5, 5))) for y in g.sample(range(nb), 2)]); rlo.append(float(g.randint(0, 6))); rup.append(inf)
    for _ in range(g.randint(0, 2)):
        S = g.sample(range(nb), g.randint(3, min(nb, 7)))
        w = [float(g.randint(1, 9)) for _ in S]
        rows.append(list(zip(S, w))); rlo.append(-inf); rup.append(float(g.randint(int(max(w)), int(sum(w)))))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); st = {}
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    st[hmap] = st.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0), st)
