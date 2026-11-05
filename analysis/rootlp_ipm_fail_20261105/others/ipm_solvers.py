"""書き出した根の LP (../mps/<問題>.mps) を他の内点法ソルバーで解く: HiGHS の IPX (クロスオーバーあり・なし)、Clarabel、PIQP、CVXOPT。
反復数・時間・目的値・最大の双対の大きさ (|y|・境界の乗数) を記録する。
使い方: python -I ipm_solvers.py 出力.jsonl 時間の上限 問題 ソルバー   (1 回に 1 ソルバー。外から timeout をかける)"""
import json, sys, time
from pathlib import Path
import numpy as np
import scipy.sparse as sp
import highspy
HERE = Path(__file__).resolve().parent
MPS = HERE.parent / "mps"
out, tl, prob, w = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3], sys.argv[4]

def rec(**kw):
    r = {"problem": prob, "solver": w, **kw}
    with out.open("a") as f:
        f.write(json.dumps(r) + "\n")
    print(r, flush=True)

def load():
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(MPS / f"{prob}.mps"))
    lp = h.getLp(); A = lp.a_matrix_
    M = sp.csc_matrix((np.array(A.value_), np.array(A.index_), np.array(A.start_)), shape=(lp.num_row_, lp.num_col_)).tocsr()
    return np.array(lp.col_cost_), np.array(lp.col_lower_), np.array(lp.col_upper_), M, np.array(lp.row_lower_), np.array(lp.row_upper_)

def resid(x, c, lo, up, A, rl, ru):
    ax = A @ x
    pv = max(np.max(np.maximum(rl - ax, 0), initial=0), np.max(np.maximum(ax - ru, 0), initial=0),
             np.max(np.maximum(lo - x, 0), initial=0), np.max(np.maximum(x - up, 0), initial=0))
    return float(pv), float(c @ x)

if w in ("ipx", "ipx_noxo"):
    h = highspy.Highs()
    h.setOptionValue("output_flag", True); h.setOptionValue("log_to_console", False)
    h.setOptionValue("log_file", str(HERE / f"{prob}_{w}.log"))
    h.setOptionValue("presolve", "off"); h.setOptionValue("time_limit", tl); h.setOptionValue("threads", 3)
    h.setOptionValue("solver", "ipm"); h.setOptionValue("run_crossover", "on" if w == "ipx" else "off")
    h.readModel(str(MPS / f"{prob}.mps"))
    t0 = time.time(); h.run(); t = time.time() - t0
    i = h.getInfo(); s = h.getSolution()
    rec(status=h.modelStatusToString(h.getModelStatus()), obj=i.objective_function_value, t=round(t, 3), iters=i.ipm_iteration_count,
        xo_iters=i.crossover_iteration_count, max_pinf=i.max_primal_infeasibility, max_dinf=i.max_dual_infeasibility,
        max_abs_y=float(np.abs(np.array(s.row_dual)).max()), max_abs_rc=float(np.abs(np.array(s.col_dual)).max()))
    sys.exit()

c, lo, up, A, rl, ru = load()
n = len(c)
E = rl == ru
mu = (~E) & (ru < 1e20); ml = (~E) & (rl > -1e20)
if w == "clarabel":
    import clarabel
    I = sp.identity(n, format="csr"); cu = up < 1e20; cl = lo > -1e20
    Ain = sp.vstack([A[mu], -A[ml], I[cu], -I[cl]]).tocsc(); bin_ = np.concatenate([ru[mu], -rl[ml], up[cu], -lo[cl]])
    Af = sp.vstack([A[E], Ain]).tocsc(); bf = np.concatenate([ru[E], bin_])
    st = clarabel.DefaultSettings(); st.verbose = False; st.time_limit = tl; st.max_iter = 500
    t0 = time.time(); sol = clarabel.DefaultSolver(sp.csc_matrix((n, n)), c, Af, bf, [clarabel.ZeroConeT(int(E.sum())), clarabel.NonnegativeConeT(Ain.shape[0])], st).solve(); t = time.time() - t0
    z = np.array(sol.z); x = np.array(sol.x); pv, ob = resid(x, c, lo, up, A, rl, ru)
    rec(status=str(sol.status), obj=sol.obj_val, t=round(t, 3), iters=sol.iterations, max_pinf=pv,
        max_abs_y=float(np.abs(z[: int(E.sum()) + int(mu.sum()) + int(ml.sum())]).max()), max_abs_rc=float(np.abs(z[int(E.sum()) + int(mu.sum()) + int(ml.sum()):]).max()))
elif w == "piqp":
    import piqp
    NE = ~E
    inf = lambda v, big: np.where(np.abs(v) < 1e20, v, np.inf if big > 0 else -np.inf)
    s = piqp.SparseSolver(); s.settings.verbose = False; s.settings.max_iter = 500
    s.setup(sp.csc_matrix((n, n)), c, A[E].tocsc(), ru[E], A[NE].tocsc(), inf(rl[NE], -1), inf(ru[NE], 1), inf(lo, -1), inf(up, 1))
    t0 = time.time(); st = s.solve(); t = time.time() - t0
    r = s.result; x = np.array(r.x); pv, ob = resid(x, c, lo, up, A, rl, ru)
    def mx(*names):
        v = [np.abs(np.array(getattr(r, k)))[np.isfinite(np.array(getattr(r, k)))].max(initial=0) for k in names if hasattr(r, k)]
        return float(max(v, default=0))
    rec(status=str(st), obj=ob, t=round(t, 3), iters=r.info.iter, max_pinf=pv, max_abs_y=mx("y", "z_l", "z_u", "z"), max_abs_rc=mx("z_bl", "z_bu", "z_lb", "z_ub"))
elif w == "cvxopt":
    from cvxopt import matrix, spmatrix, solvers
    def cv(M):
        M = M.tocoo(); return spmatrix(M.data.tolist(), M.row.tolist(), M.col.tolist(), size=M.shape)
    I = sp.identity(n, format="csr"); cu = up < 1e20; cl = lo > -1e20
    G = sp.vstack([A[mu], -A[ml], I[cu], -I[cl]]); hh = np.concatenate([ru[mu], -rl[ml], up[cu], -lo[cl]])
    solvers.options["show_progress"] = False; solvers.options["maxiters"] = 200
    t0 = time.time()
    sol = solvers.lp(matrix(c), cv(G), matrix(hh), cv(A[E]), matrix(ru[E])) if E.any() else solvers.lp(matrix(c), cv(G), matrix(hh))
    t = time.time() - t0
    x = np.array(sol["x"]).ravel(); pv, ob = resid(x, c, lo, up, A, rl, ru)
    rec(status=sol["status"], obj=ob, t=round(t, 3), iters=sol["iterations"], max_pinf=pv,
        max_abs_y=float(max(np.abs(np.array(sol["y"])).max(initial=0), np.abs(np.array(sol["z"])[: int(mu.sum() + ml.sum())]).max(initial=0))),
        max_abs_rc=float(np.abs(np.array(sol["z"])[int(mu.sum() + ml.sum()):]).max(initial=0)), gap=sol["relative gap"])
