"""クリーク型の行 (集合詰め込み・分割) の多い乱数の 0-1 混合問題を作り、HiGHS と最適値を突き合わせる
(python scripts/mip_fuzz_clique.py 問題数 [開始seed])。試作 `ENOMOTO_MIP_CLQ_LAZY` (余裕のあるクリーク型の行を LP から外す) の確かめ用。
否定の文字 (1 - x) を含む行・ナップサック行・一般整数と連続の列も混ぜ、木の探索に入る大きさ (2 値 60〜150 列) にする。"""
import random, sys, time, highspy
from enomoto_solver import Model
inf = float('inf')
def gen(seed):
    g = random.Random(seed); nb = g.randint(60, 150); ng = g.randint(0, 4); nc = g.randint(0, 4); n = nb + ng + nc
    lo = [0.0] * nb + [0.0] * ng + [0.0] * nc
    up = [1.0] * nb + [float(g.randint(2, 6)) for _ in range(ng)] + [float(g.randint(3, 10)) for _ in range(nc)]
    it = [True] * (nb + ng) + [False] * nc
    c = [round(-g.random() * 10 - (2 if j < nb else 0), 2) for j in range(n)]
    # 集合詰め込みの行 (否定の文字は 1 つまで) は 2 値が全部 0 の解で満たされる。分割の行は、1 にしても詰め込みの行を破らない列を 1 つ選んで満たす
    x0 = [0.0] * nb + [float(g.randint(0, int(up[j]))) for j in range(nb, nb + ng)] + [g.random() * up[j] for j in range(nb + ng, n)]
    rows, rlo, rup = [], [], []
    pack = []
    for _ in range(g.randint(2 * nb, 5 * nb)):  # 集合詰め込み
        S = g.sample(range(nb), g.randint(2, min(5, nb)))
        neg = [S[0]] if g.random() < 0.2 else []
        pack.append((S, neg)); rows.append([(j, -1.0 if j in neg else 1.0) for j in S]); rlo.append(-inf); rup.append(1.0 - len(neg))
    ok = lambda: all(sum((1.0 - x0[j]) if j in neg else x0[j] for j in S) <= 1.0 for S, neg in pack)
    for _ in range(g.randint(0, 3)):  # 集合分割
        S = g.sample(range(nb), g.randint(2, min(6, nb)))
        if any(x0[j] == 1.0 for j in S): continue
        for j in S:
            x0[j] = 1.0
            if ok(): break
            x0[j] = 0.0
        else:
            continue
        rows.append([(j, 1.0) for j in S]); rlo.append(1.0); rup.append(1.0)
    for _ in range(g.randint(1, 4)):  # ナップサック (一般整数・連続も)
        S = g.sample(range(n), g.randint(3, min(10, n)))
        a = [(j, float(g.randint(1, 9))) for j in S]
        act = sum(v * x0[j] for j, v in a)
        rows.append(a); rlo.append(-inf); rup.append(float(int(act + g.randint(0, 6))))
    for j in range(nb, n):  # 一般整数・連続を 2 値に結ぶ (x_j <= up_j * y、x0 で 1 の y か、x0_j = 0 なら任意)
        cand = [y for y in range(nb) if x0[y] == 1.0] if x0[j] > 0 else list(range(nb))
        if not cand: x0[j] = 0.0; cand = list(range(nb))
        if not cand: continue
        y = g.choice(cand); rows.append([(j, 1.0), (y, -up[j])]); rlo.append(-inf); rup.append(0.0)
    return lo, up, c, it, rows, rlo, rup
def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup): h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value
def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [M.add_variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
