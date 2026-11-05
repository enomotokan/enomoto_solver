"""他の内点法の解を、MIP の根の LP の標準形 (TwoStageLp::build_std: 列は x' = x - shift、行 i は σ_i a_i x' + s_i = b_i) の点
(x' と s を n + m 個、y を m 個、f64 リトルエンディアン) に写して書く。ENOMOTO_DEBUG_XO_POINT_LOAD で読ませ、こちらのクロスオーバーだけを走らせる。
使い方: python -I make_point.py 問題 ソルバー(ipx_noxo|clarabel) 出力.bin"""
import sys, time
from pathlib import Path
import numpy as np
import scipy.sparse as sp
import highspy
HERE = Path(__file__).resolve().parent
prob, w, outf = sys.argv[1], sys.argv[2], sys.argv[3]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(HERE.parent / "mps" / f"{prob}.mps"))
lp = h.getLp(); Am = lp.a_matrix_
A = sp.csc_matrix((np.array(Am.value_), np.array(Am.index_), np.array(Am.start_)), shape=(lp.num_row_, lp.num_col_)).tocsr()
c = np.array(lp.col_cost_); lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_); rl = np.array(lp.row_lower_); ru = np.array(lp.row_upper_)
n, m = len(c), len(rl)
INF = 1e20
t0 = time.time()
if w == "ipx_noxo":
    h.setOptionValue("presolve", "off"); h.setOptionValue("solver", "ipm"); h.setOptionValue("run_crossover", "off"); h.setOptionValue("threads", 3)
    h.run(); s = h.getSolution(); x = np.array(s.col_value); y = np.array(s.row_dual)
else:
    import clarabel
    E = rl == ru; mu = (~E) & (ru < INF); ml = (~E) & (rl > -INF)
    I = sp.identity(n, format="csr"); cu = up < INF; cl = lo > -INF
    Af = sp.vstack([A[E], A[mu], -A[ml], I[cu], -I[cl]]).tocsc(); bf = np.concatenate([ru[E], ru[mu], -rl[ml], up[cu], -lo[cl]])
    st = clarabel.DefaultSettings(); st.verbose = False
    sol = clarabel.DefaultSolver(sp.csc_matrix((n, n)), c, Af, bf, [clarabel.ZeroConeT(int(E.sum())), clarabel.NonnegativeConeT(Af.shape[0] - int(E.sum()))], st).solve()
    x = np.array(sol.x); z = np.array(sol.z); y = np.zeros(m)
    k = 0
    y[np.where(E)[0]] = -z[k:k + E.sum()]; k += E.sum()
    y[np.where(mu)[0]] -= z[k:k + mu.sum()]; k += mu.sum()
    y[np.where(ml)[0]] += z[k:k + ml.sum()]
print(f"{w}: {time.time() - t0:.2f}s obj={c @ x:.10e} dual resid (c - A^T y の境界と矛盾する符号の最大) を後で見る")
shift = np.where(lo > -INF, lo, np.where(up < INF, up, 0.0))
ax = A @ x
sig = np.where(ru < INF, 1.0, np.where(rl > -INF, -1.0, 1.0))
sl = np.where(ru < INF, ru - ax, np.where(rl > -INF, ax - rl, 0.0))
xs = np.concatenate([x - shift, sl])
ys = sig * y
np.concatenate([xs, ys]).astype("<f8").tofile(outf)
d = c - A.T @ y
bad = np.maximum(np.where(lo > -INF, 0, np.maximum(d, 0)), 0) + np.where(up < INF, 0, np.maximum(-d, 0))
print("n", n, "m", m, "max|y|", np.abs(y).max(), "dual sign violation on infinite sides", bad.max())
