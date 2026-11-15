"""固定費用の輸送問題 (連続の流れ x_ij <= u_ij y_ij、y は 0-1) と一般整数の在庫を少し混ぜた乱数の MIP を作り、HiGHS と最適値を
突き合わせる (試作 ENOMOTO_MIP_NODECNT のノードの経路集約・退化した双対の固定が働く形)。
使い方: ENOMOTO_MIP_NODECNT=dh ENOMOTO_MIP_REL_GAP=1e-9 python fuzz_fc.py 問題数 [開始seed]"""
import importlib.util, random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("fz", str(WT / "scripts/mip_fuzz_vs_highs.py"))
src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec.origin, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); S = g.randint(5, 10); T = g.randint(10, 20)
    arcs = [(i, j) for i in range(S) for j in range(T) if g.random() < 0.8]
    na = len(arcs)
    lo = [0.0] * (2 * na); up = []; c = []; it = []
    for (i, j) in arcs:
        up.append(inf); c.append(float(g.randint(1, 9))); it.append(g.random() < 0.15)
    for _ in arcs:
        up.append(1.0); c.append(float(g.randint(5, 60))); it.append(True)
    dem = [g.randint(2, 15) for _ in range(T)]
    sup = [g.randint(5, 30) for _ in range(S)]
    if sum(sup) < sum(dem):
        sup[0] += sum(dem) - sum(sup) + g.randint(0, 5)
    rows = []; rlo = []; rup = []
    for i in range(S):
        r = [(k, 1.0) for k, (a, b) in enumerate(arcs) if a == i]
        if r: rows.append(r); rlo.append(-inf); rup.append(float(sup[i]))
    for j in range(T):
        r = [(k, 1.0) for k, (a, b) in enumerate(arcs) if b == j]
        if r: rows.append(r); rlo.append(float(dem[j])); rup.append(inf if g.random() < 0.5 else float(dem[j]))
    for k, (a, b) in enumerate(arcs):
        u = float(min(sup[a], dem[b]) + g.randint(0, 3)) * g.choice([1.0, 1.0, 0.5])
        rows.append([(k, 1.0), (na + k, -u)]); rlo.append(-inf); rup.append(0.0)
    for _ in range(g.randint(0, 2)):  # 開く弧の数の上限
        ks = g.sample(range(na), max(2, na // 3)); rows.append([(na + k, 1.0) for k in ks]); rlo.append(-inf); rup.append(float(g.randint(len(ks) // 2, len(ks))))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); stat = {}
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    stat[hmap] = stat.get(hmap, 0) + 1
    if not ok:
        bad += 1; print("MISMATCH", seed, hs, ho, os_, oo, flush=True)
print(f"done {N} bad {bad} {time.time() - t0:.1f}s {stat}")
