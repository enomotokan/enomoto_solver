"""試作 ENOMOTO_MIP_VB01 を狙った fuzz: 0-1 列だけで、含意 (x <= y)・AND 型 (x + y - z <= 1、x + y - 2z >= 0)・集合分割/詰め込み・
ナップサックの行を持つ小さな MIP を作り、HiGHS と最適値を比べる。python fuzz_vb01.py 問題数 [開始seed]
(ENOMOTO_MIP_VB01=1・ENOMOTO_MIP_LOG=1 を付けて回すと、VB01 が根で回った数・解を出した数も数える。比較は ENOMOTO_MIP_REL_GAP=1e-9 で)"""
import os, random, sys, tempfile, time
import highspy
from enomoto_solver import Model, Variable
inf = float("inf")

def gen(seed):
    g = random.Random(seed); n = g.randint(40, 250)
    x0 = [g.randrange(2) for _ in range(n)]
    c = [float(g.choice([0, 1, 2, -1, -2, -3, -5, -10])) for _ in range(n)]
    rows = []  # (係数の組, 下限, 上限)
    for _ in range(g.randint(2 * n, 4 * n)):
        t = g.randrange(7)
        if t <= 1:  # 含意 x <= y (x0 で満たすもの)
            i, j = g.sample(range(n), 2)
            if x0[i] <= x0[j]: rows.append(([(i, 1.0), (j, -1.0)], -inf, 0.0))
        elif t == 2:  # AND: z = x and y
            i, j, k = g.sample(range(n), 3)
            if x0[k] == (x0[i] & x0[j]):
                rows.append(([(i, 1.0), (j, 1.0), (k, -1.0)], -inf, 1.0)); rows.append(([(i, 1.0), (j, 1.0), (k, -2.0)], 0.0, inf))
        elif t == 3:  # 集合詰め込み・分割
            S = g.sample(range(n), g.randint(2, min(8, n)))
            s = sum(x0[j] for j in S)
            if s <= 1: rows.append(([(j, 1.0) for j in S], 1.0 if (s == 1 and g.random() < 0.5) else -inf, 1.0))
        elif t in (4, 6):  # ナップサック
            S = g.sample(range(n), g.randint(5, min(30, n)))
            a = [(j, float(g.randint(1, 9))) for j in S]
            act = sum(v * x0[j] for j, v in a)
            rows.append((a, -inf, act + g.randrange(4)))
        else:  # 被覆
            S = g.sample(range(n), g.randint(2, min(6, n)))
            if sum(x0[j] for j in S) >= 1: rows.append(([(j, 1.0) for j in S], 1.0, inf))
    return n, c, rows

def highs(lp):
    n, c, rows = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    K = highspy.kHighsInf
    for j in range(n):
        h.addVar(0, 1); h.changeColCost(j, c[j]); h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in rows:
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value

def ours(lp):
    n, c, rows = lp
    M = Model(); xs = [Variable(int, 0, 1) for _ in range(n)]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in rows:
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective

N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = ran = found = 0; t0 = time.time()
tmp = tempfile.NamedTemporaryFile(delete=False); tmp.close()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp)
    # 当方の stderr (ログ) を一時ファイルに向けて VB01 の行を数える
    sys.stderr.flush(); saved = os.dup(2); fd = os.open(tmp.name, os.O_WRONLY | os.O_TRUNC); os.dup2(fd, 2); os.close(fd)
    try:
        os_, oo = ours(lp)
    finally:
        os.dup2(saved, 2); os.close(saved)
    log = open(tmp.name, errors="replace").read()
    ran += "fixing rate" in log and "VB01" in log
    found += "root heuristic VB01: found" in log
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "VB01 ran", ran, "found", found, "time %.1f" % (time.time() - t0))
