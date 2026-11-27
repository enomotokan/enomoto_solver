"""一般の不等式の行 (ナップサック・変数上下界の 2 列・整数と連続の混ざり・連続だけ・クリーク型・1 列) の多い乱数の混合整数問題を作り、
HiGHS と最適値を突き合わせる (python scripts/mip_fuzz_genlazy.py 問題数 [開始seed])。試作 `ENOMOTO_MIP_GEN_LAZY` (余裕のある
不等式の行を LP から外し、破られたら戻す) の確かめ用。木の探索に入る大きさ (列 30〜90) にし、範囲の無限な連続列も混ぜる。
実行可能な点 x0 を先に決め、行の右辺は x0 での活動に余裕を足して決める (等式は少しだけ)。"""
import random, sys, time, highspy
from enomoto_solver import Model
inf = float('inf')


def gen(seed):
    g = random.Random(seed)
    nb = g.randint(40, 110); ng = g.randint(3, 20); nc = g.randint(5, 30); n = nb + ng + nc
    lo, up, it = [], [], []
    for j in range(n):
        if j < nb: lo.append(0.0); up.append(1.0); it.append(True)
        elif j < nb + ng: lo.append(float(-g.randint(0, 3))); up.append(float(g.randint(2, 12))); it.append(True)
        else:
            t = g.randrange(4)
            lo.append(0.0 if t else -float(g.randint(1, 5))); up.append(inf if t == 1 else float(g.randint(2, 20))); it.append(False)
    x0 = []
    for j in range(n):
        u = up[j] if up[j] < inf else lo[j] + 10
        v = lo[j] + (u - lo[j]) * g.random()
        x0.append(float(round(v)) if it[j] else v)
    c = [round((g.random() - 0.7) * 20, 2) for j in range(n)]
    for j in range(n):  # 範囲の無限な列は目的を正にして有界にする
        if up[j] == inf: c[j] = abs(c[j]) + 0.5
    rows, rlo, rup = [], [], []
    def add(r, slack_hi, eq=False):
        act = sum(a * x0[j] for j, a in r)
        if eq:
            rows.append(r); rlo.append(act); rup.append(act); return
        s = g.random() * slack_hi if g.random() < 0.7 else 0.0
        if g.random() < 0.5: rows.append(r); rlo.append(-inf); rup.append(act + s)
        else: rows.append(r); rlo.append(act - s); rup.append(inf)
    B, I, C = list(range(nb)), list(range(nb, nb + ng)), list(range(nb + ng, n))
    for _ in range(g.randint(nb // 2, 2 * nb)):  # クリーク型 (x0 で満たす)
        S = g.sample(B, g.randint(2, 5))
        if sum(x0[j] for j in S) <= 1: rows.append([(j, 1.0) for j in S]); rlo.append(-inf); rup.append(1.0)
    for _ in range(g.randint(5, 25)):  # ナップサック (2 値)
        S = g.sample(B, g.randint(3, min(12, nb))); add([(j, float(g.randint(1, 30))) for j in S], 20)
    for _ in range(g.randint(2, 10)):  # 整数だけ (一般の整数を含む)
        S = g.sample(B + I, g.randint(2, 8)); add([(j, float(g.randint(-9, 9) or 1)) for j in S], 8)
    for _ in range(g.randint(3, 20)):  # 変数上下界 (2 列)
        x = g.choice(C + I); y = g.choice(B); u = (up[x] if up[x] < inf else 30.0)
        r = [(x, 1.0), (y, -u)]
        if x0[y] == 0 and x0[x] > 0: r = [(x, 1.0), (y, -(u + 1))]
        add(r, 3)
    for _ in range(g.randint(3, 15)):  # 整数と連続の混ざり
        S = g.sample(B + I, g.randint(1, 5)) + g.sample(C, g.randint(1, min(5, nc)))
        add([(j, round((g.random() - 0.5) * 20, 1) or 1.0) for j in S], 10)
    for _ in range(g.randint(1, 8)):  # 連続だけ
        S = g.sample(C, g.randint(2, min(6, nc))); add([(j, round((g.random() - 0.5) * 10, 1) or 1.0) for j in S], 5)
    for _ in range(g.randint(0, 4)):  # 1 列
        j = g.randrange(n); add([(j, float(g.randint(1, 4)))], 2)
    for _ in range(g.randint(0, 2)):  # 等式
        S = g.sample(range(n), g.randint(2, 5)); add([(j, float(g.randint(1, 5))) for j in S], 0, eq=True)
    return lo, up, c, it, rows, rlo, rup


def highs(lp, presolve=True):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    if not presolve: h.setOptionValue('presolve', 'off')
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
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


if __name__ == '__main__':
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; t0 = time.time()
    for seed in range(start, start + N):
        lp = gen(seed)
        hs, ho = highs(lp); os_, oo = ours(lp)
        hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
        ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if not ok:  # HiGHS の前処理の誤り (seed 157 で前処理ありの HiGHS が実行可能な問題を Infeasible とした) を除くため、前処理なしの HiGHS でも比べる
            hs2, ho2 = highs(lp, False)
            hmap2 = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs2, hs2)
            if (hmap2 == os_) and (hmap2 != 'optimal' or abs(ho2 - oo) <= 1e-6 * (1 + abs(ho2))):
                print("seed", seed, "highs (presolve)", hs, ho, "differs from highs (no presolve)", hs2, ho2, "= ours", os_, oo, flush=True)
                ok = True
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
    print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
