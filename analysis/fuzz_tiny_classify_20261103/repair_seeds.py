"""fuzz_mixtiny の修復が起きる seed を ENOMOTO (結果の値・下界) と HiGHS で解き比べる。
使い方: ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FZ3_REPAIR_BOUND=1 python analysis/fuzz_tiny_classify_20261103/repair_seeds.py seed...
(Python の Solution から best_bound を読む。ログの 'postsolve repair' 行に新旧の下界のずれが出る)"""
import sys
FM = "benchmarks/tinywidth_20261022/fuzz_mixtiny.py"
src = open(FM).read().rsplit("N=int(sys.argv[1])", 1)[0]
ns = {"__file__": FM}; exec(compile(src, FM, "exec"), ns)
from enomoto_solver import Model, Variable
inf = float("inf")
for seed in map(int, sys.argv[1:]):
    lo, up, c, it, rows, rlo, rup = lp = ns["gen"](seed); hs, ho = ns["highs"](lp)
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    try:
        s = M.solve(mip_rel_gap=1e-9)
        bb = getattr(s, "best_bound", None)
        print("RES seed", seed, "highs", hs, repr(ho), "ours", s.status, repr(s.objective), "bound", repr(bb),
              "bound-highs %.3e" % (bb - ho) if bb is not None and ho is not None else "", flush=True)
    except BaseException as e:
        print("RES seed", seed, "PANIC", str(e)[:60], flush=True)
