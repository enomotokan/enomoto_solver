"""RENS の固定の選び方を変えた近傍を HiGHS で解く: python rensvar.py <接頭辞k> [秒]
all: LP で整数値の自由な整数列を全部固定 (ENOMOTO の旧来の RENS)。
adj: そのうち、分数の列と同じ行に入る列は固定しない。
adj0: adj のうち、目的の係数が 0 の列だけ固定しない。
いずれも分数の列は [floor, ceil] に狭める。"""
import sys, time, numpy as np, highspy, scipy.sparse as sp
pre = sys.argv[1]; T = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
def load():
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(pre + ".mps"); return h
h = load(); lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_; M = sp.csc_matrix((A.value_, A.index_, A.start_), shape=(m, n)); R = M.tocsr()
lo, up, c = np.array(lp.col_lower_), np.array(lp.col_upper_), np.array(lp.col_cost_)
isint = np.array([int(v) for v in lp.integrality_]) == 1
x = np.loadtxt(pre + "_x.txt")
free = isint & (lo < up)
frac = free & (np.abs(x - np.round(x)) > 1e-6)
integral = free & ~frac
# 分数の列を含む行
rowfrac = (np.abs(R) @ frac.astype(float)) > 0
adj = (np.abs(M).T @ rowfrac.astype(float)) > 0
sets = {"all": integral, "adj": integral & ~adj, "adj0": integral & ~(adj & (c == 0))}
print(f"{pre}: free integer {free.sum()}, fractional {frac.sum()}, LP-integral {integral.sum()}, LP-integral sharing a row with a fractional column {np.sum(integral & adj)} (cost 0: {np.sum(integral & adj & (c == 0))})")
for k, fix in sets.items():
    g = load(); g.setOptionValue("threads", 4); g.setOptionValue("time_limit", T)
    nl, nu = lo.copy(), up.copy()
    nl[fix] = np.round(x[fix]); nu[fix] = np.round(x[fix])
    nl[frac] = np.maximum(nl[frac], np.floor(x[frac])); nu[frac] = np.minimum(nu[frac], np.ceil(x[frac]))
    g.changeColsBounds(n, np.arange(n, dtype=np.int32), nl, nu)
    t = time.perf_counter(); g.run()
    st = g.modelStatusToString(g.getModelStatus()); inf = g.getInfo()
    obj = inf.objective_function_value if inf.primal_solution_status else None
    print(f"  {k}: fixed {fix.sum()} ({fix.sum() / free.sum():.2f} of free), free integer {free.sum() - fix.sum()}: {st}, objective {obj}, nodes {inf.mip_node_count} ({time.perf_counter() - t:.1f}s)")
