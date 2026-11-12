"""費用 0 の連続列 (スラック列) と範囲制約・行にも目的にも出ない列を含む小さな MIP を乱数で作り、HiGHS と状態・最適値を突き合わせる
(2026-11-12、analysis/wrongans_20261112.md)。行の 1/8 は区間の空な範囲制約にする (L > U) ので、実行不能な問題も混ざる。
使い方: python fuzz_slack.py 問題数 [開始seed]   (ENOMOTO_MIP_THREADS は既定 1、ENOMOTO_MIP_REL_GAP=1e-9 を置く)"""
import math, os, random, sys
os.environ.setdefault("ENOMOTO_MIP_THREADS", "1")
os.environ.setdefault("ENOMOTO_MIP_REL_GAP", "1e-9")
import highspy
from enomoto_solver import Model, Variable
inf = math.inf


def gen(seed):
    g = random.Random(seed)
    n = g.randint(3, 14)
    lo, up, c, it = [], [], [], []
    for j in range(n):
        t = g.randrange(4)
        if t == 0: l, u = 0.0, 1.0
        elif t == 1: l = float(g.randint(-5, 3)); u = l + g.randint(1, 60)
        elif t == 2: l, u = float(g.randint(-5, 2)), inf
        else: l, u = 0.0, float(g.randint(1, 5))
        lo.append(l); up.append(u); it.append(g.random() < 0.6)
        c.append(0.0 if (not it[j] and g.random() < 0.7) or g.random() < 0.3 else float(g.randint(-9, 9)))
    m = g.randint(1, 6)
    rows, rlo, rup = [], [], []
    for _ in range(m):
        k = g.randint(1, min(n, 5))
        cols = g.sample(range(n), k)
        r = sorted((j, float(g.choice([-1, 1]) * g.choice([1, 1, 2, 3, 4.5, 7.5]))) for j in cols)
        t = g.randrange(8)
        a = float(g.randint(-10, 10))
        if t in (0, 1): l, u = a, inf
        elif t in (2, 3): l, u = -inf, a
        elif t <= 6: l, u = a, a + g.randint(0, 8)
        else: l, u = a, a - g.randint(1, 4)  # 区間が空の範囲制約
        rows.append(r); rlo.append(l); rup.append(u)
    # 行にも目的にも出ない列
    extra = g.randint(0, 6)
    for _ in range(extra):
        lo.append(0.0); up.append(float(g.randint(1, 5))); c.append(0.0); it.append(g.random() < 0.7)
    return lo, up, c, it, rows, rlo, rup


def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    n = len(lo)
    for j in range(n):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        if l > u:  # HiGHS は空の区間の行を受け付けないので、2 本に分ける
            h.addRow(l, inf, len(r), [j for j, _ in r], [v for _, v in r]); h.addRow(-inf, u, len(r), [j for j, _ in r], [v for _, v in r])
        else:
            h.addRow(l, u, len(r), [j for j, _ in r], [v for _, v in r])
    h.run()
    s = h.modelStatusToString(h.getModelStatus())
    return s, (h.getInfo().objective_function_value if s == "Optimal" else None)


def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)), "minimize")
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l > -inf: M.add_constraint(e >= l)
        if u < inf: M.add_constraint(e <= u)
    fd = os.open(os.devnull, os.O_WRONLY); s1, s2 = os.dup(1), os.dup(2); os.dup2(fd, 1); os.dup2(fd, 2)
    try: s = M.solve(mip_rel_gap=1e-9)
    finally: os.dup2(s1, 1); os.dup2(s2, 2); os.close(fd)
    return s.status, s.objective


N = int(sys.argv[1]); s0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; cnt = {}
for seed in range(s0, s0 + N):
    lp = gen(seed)
    hs, ho = highs(lp)
    os_, oo = ours(lp)
    cnt[hs] = cnt.get(hs, 0) + 1
    ok = (hs == "Optimal" and os_ == "optimal" and abs(ho - oo) <= 1e-6 * max(1, abs(ho))) or (hs == "Infeasible" and os_ == "infeasible") \
        or (hs in ("Unbounded", "Primal infeasible or unbounded") and os_ in ("unbounded", "infeasible_or_unbounded", "infeasible"))
    if not ok:
        bad += 1
        print("MISMATCH seed", seed, "HiGHS", hs, ho, "ENOMOTO", os_, oo, flush=True)
print(f"done {N} (seeds {s0}..{s0 + N - 1}): mismatches {bad}, HiGHS statuses {cnt}")
