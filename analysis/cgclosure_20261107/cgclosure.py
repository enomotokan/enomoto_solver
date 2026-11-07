"""ランク 1 の CG 閉包 (と mod-k・mod-2 の閉包) の下界を、Fischetti–Lodi (2007) の分離 MIP で近似する (2026-11-07、analysis/cgclosure_20261107.md)。

使い方: python -I cgclosure.py 問題.mps モード [--tsep 秒] [--rounds 回] [--tmax 秒] [--sol 既知解.sol] [--out 出力.jsonl] [--name 名前]
  モード: cg (u ∈ [0, 1-δ] の連続)、mod2 (u ∈ {0, 1/2})、modk (u ∈ {0, 1/k, ..., (k-1)/k}、k = 2, 3, 5, 7 をラウンドごとに全部)、
          modK<k> (k を 1 つ、例 modK3)

手順 (元の模型、前処理なし):
- 列を x = d + σ x' (x' ≥ 0) に移す (整数列は下限があれば下限、なければ上限から測る。両方ない整数列は uA_j が整数になる条件にする)。
- 行は ≤ の形 (等号・両側の行は 2 本)。整数列の係数を有理数として通分し、整数列の係数の最大公約数で割る (整数にできない行はそのまま)。
  整数列の上限 x' ≤ u - l も行にする。
- 連続列は Fischetti–Lodi・射影 CG (Bonami ほか 2008) のとおり、uA_j (+ 連続列の上限の行の乗数 v_j ≥ 0) ≥ 0 を課して係数 0 で落とす。
- 分離 MIP: max Σ_{j: x'*_j>0} α_j x'*_j - β - ε Σ u_i、α_j ≤ uA_j、β ≥ ub + Σ v_j W_j - 1 + δ、α・β 整数。
  使う行は LP 点での余裕 (尺度を直した行で) が 1 未満のもの。x'*_j = 0 の整数列の α は持たない (カットでは ⌊uA_j⌋ を後で計算する)。
- 見つけた解 (改善解を全部) からカットを浮動小数で作り直し、違反するものを LP に足して解き直す。
"""
import argparse, gzip, json, math, sys, time
from fractions import Fraction
import numpy as np
import scipy.sparse as sp
import highspy

INF = highspy.kHighsInf
DELTA = 0.01
EPS_U = 1e-5
MINVIOL = 1e-3
UBIG = 10.0  # 整数にできない行・連続列を含む行の乗数の上限 (近似)
SMAX = float("inf")  # 余裕で行を絞らない (1 未満に絞ると p0201 で閉包を大きく取りこぼした)


def int_scale(vals):
    """整数列の係数 vals を整数にする正の倍率 (通分して最大公約数で割る)。できなければ None。"""
    if len(vals) == 0:
        return None
    den = 1
    for v in vals:
        f = Fraction(v).limit_denominator(1000)
        if abs(float(f) - v) > 1e-9 * max(1.0, abs(v)):
            return None
        den = den * f.denominator // math.gcd(den, f.denominator)
        if den > 10**6:
            return None
    ints = [round(v * den) for v in vals]
    g = 0
    for a in ints:
        g = math.gcd(g, abs(a))
    if g == 0:
        return None
    return den / g


class Model:
    def __init__(self, path):
        h = highspy.Highs()
        h.setOptionValue("output_flag", False)
        h.readModel(path)
        lp = h.getLp()
        self.n, self.m0 = lp.num_col_, lp.num_row_
        self.sense = 1.0 if lp.sense_ == highspy.ObjSense.kMinimize else -1.0
        self.c = np.array(lp.col_cost_) * self.sense
        self.offset = lp.offset_ * self.sense
        self.lo = np.array(lp.col_lower_, dtype=float)
        self.up = np.array(lp.col_upper_, dtype=float)
        integ = lp.integrality_ if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * self.n
        self.isint = np.array([t != highspy.HighsVarType.kContinuous for t in integ])
        for j in np.where(self.isint)[0]:
            if self.lo[j] > -INF:
                self.lo[j] = math.ceil(self.lo[j] - 1e-9)
            if self.up[j] < INF:
                self.up[j] = math.floor(self.up[j] + 1e-9)
        A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(self.m0, self.n))
        self.A = A.tocsr()
        self.rl = np.array(lp.row_lower_, dtype=float)
        self.ru = np.array(lp.row_upper_, dtype=float)
        # 列の変換 x = d + σ x'
        self.sig = np.zeros(self.n)
        self.d = np.zeros(self.n)
        self.W = np.full(self.n, INF)  # x' の上限
        for j in range(self.n):
            if self.lo[j] > -INF:
                self.sig[j], self.d[j] = 1.0, self.lo[j]
                if self.up[j] < INF:
                    self.W[j] = self.up[j] - self.lo[j]
            elif self.up[j] < INF:
                self.sig[j], self.d[j] = -1.0, self.up[j]
            else:
                self.sig[j], self.d[j] = 0.0, 0.0  # 自由列
        self.build_rows()

    def build_rows(self):
        rows, cols, vals, rhs, pure = [], [], [], [], []
        nscaled = nfail = 0
        A, sig, d = self.A, self.sig, self.d
        r = 0
        for i in range(self.m0):
            s, e = A.indptr[i], A.indptr[i + 1]
            idx, a = A.indices[s:e], A.data[s:e]
            # 自由列は σ = 1 として扱う (x' = x、符号の制約なし)
            sg = np.where(sig[idx] == 0, 1.0, sig[idx])
            at = a * sg
            shift = float(np.dot(a, d[idx]))
            ii = self.isint[idx]
            for side, bound in ((1.0, self.ru[i]), (-1.0, -self.rl[i])):
                if not (bound < INF):
                    continue
                av = side * at
                b = bound - side * shift
                sc = int_scale(list(av[ii])) if ii.any() else None
                if sc is None:
                    if ii.any():
                        nfail += 1
                        mn = np.min(np.abs(av[ii]))
                    else:
                        mn = np.max(np.abs(av)) if len(av) else 1.0
                    sc = 1.0 / mn if mn > 0 else 1.0
                    pr = False
                else:
                    nscaled += 1
                    pr = bool(ii.all())
                av = av * sc
                b = b * sc
                if pr:
                    av = np.round(av)
                    b = math.floor(b + 1e-9)  # 整数の行は右辺を切り下げる
                rows += [r] * len(idx); cols += list(idx); vals += list(av); rhs.append(b); pure.append(pr)
                r += 1
        # 整数列の上限の行 x'_j ≤ W_j
        for j in np.where(self.isint & (self.W < INF) & (self.W > 0))[0]:
            rows.append(r); cols.append(j); vals.append(1.0); rhs.append(self.W[j]); pure.append(True); r += 1
        self.T = sp.csr_matrix((vals, (rows, cols)), shape=(r, self.n))
        self.Tc = self.T.tocsc()
        self.b = np.array(rhs)
        self.pure = np.array(pure)
        # 乗数の上限: 整数の行 (整数列だけ・整数係数・整数右辺) は 1 - δ (Fischetti–Lodi)、それ以外は UBIG
        self.umax = np.where(self.pure, 1.0 - DELTA, UBIG)
        self.nscaled, self.nfail = nscaled, nfail

    def to_t(self, x):
        sg = np.where(self.sig == 0, 1.0, self.sig)
        return (x - self.d) * sg


def separate(M, x, mode_k, tsep, threads=1):
    """x (元の空間) を切る CG カットを分離 MIP で探す。mode_k = None なら連続の u。返り値: [(coef(元の空間), rhs, info)]"""
    xt = M.to_t(x)
    slack = M.b - M.T @ xt
    R = np.where(slack < SMAX)[0]
    if len(R) > 20000:
        R = R[np.argsort(slack[R])[:20000]]
    TR = M.T[R]
    TRc = TR.tocsc()
    colnnz = np.diff(TRc.indptr)
    free = (M.sig == 0)
    J = np.where(M.isint & ((xt > 1e-6) | free) & (colnnz > 0))[0]
    C = np.where(~M.isint & (colnnz > 0))[0]
    Cv = C[M.W[C] < INF]
    nR, nJ, nC, nV = len(R), len(J), len(C), len(Cv)
    if nR == 0 or nJ == 0:
        return [], {"nR": nR, "nJ": nJ}
    # 変数: u (nR) | v (nV) | α (nJ) | β
    # 制約: α_j - Σ u_i a_ij ≤ 0 (自由列は = 0)  [nJ 本]
    #       Σ u_i a_ij + v_j ≥ 0 (自由な連続列は = 0) [nC 本]
    #       β - Σ u_i b_i - Σ v_j W_j ≥ -1 + δ  [1 本]
    k = mode_k
    ucoef = 1.0 / k if k else 1.0
    AJ = TRc[:, J].T.tocsr() * (-ucoef)          # nJ × nR
    AC = TRc[:, C].T.tocsr() * ucoef             # nC × nR
    bR = sp.csr_matrix(-(M.b[R] * ucoef).reshape(1, -1))
    posv = {j: t for t, j in enumerate(Cv)}
    Vc = sp.csr_matrix((np.ones(nV), ([np.where(C == j)[0][0] for j in Cv], range(nV))), shape=(nC, nV)) if nV else sp.csr_matrix((nC, 0))
    Vb = sp.csr_matrix((-M.W[Cv]).reshape(1, -1)) if nV else sp.csr_matrix((1, 0))
    blocks = [[AJ, sp.csr_matrix((nJ, nV)), sp.identity(nJ, format="csr"), sp.csr_matrix((nJ, 1))],
              [AC, Vc, sp.csr_matrix((nC, nJ)), sp.csr_matrix((nC, 1))],
              [bR, Vb, sp.csr_matrix((1, nJ)), sp.csr_matrix(np.ones((1, 1)))]]
    Ms = sp.bmat(blocks, format="csc")
    nv = nR + nV + nJ + 1
    lp = highspy.HighsLp()
    lp.num_col_, lp.num_row_ = nv, nJ + nC + 1
    cost = np.zeros(nv)
    cost[:nR] = EPS_U * ucoef
    cost[nR + nV:nR + nV + nJ] = -xt[J]
    cost[-1] = 1.0
    lb = np.zeros(nv); ub = np.zeros(nv)
    ub[:nR] = (k - 1) if k else M.umax[R]
    ub[nR:nR + nV] = INF
    # α の範囲
    TRJ = TRc[:, J]
    um = sp.diags(np.full(nR, (k - 1) / k) if k else M.umax[R])
    pos = np.asarray((um @ TRJ.maximum(0)).sum(axis=0)).ravel()
    neg = np.asarray((um @ TRJ.minimum(0)).sum(axis=0)).ravel()
    lb[nR + nV:nR + nV + nJ] = np.floor(neg) - 1
    ub[nR + nV:nR + nV + nJ] = np.ceil(pos) + 1
    lb[-1], ub[-1] = -INF, INF
    lp.col_cost_ = cost; lp.col_lower_ = lb; lp.col_upper_ = ub
    rlo = np.concatenate([np.full(nJ, -INF), np.zeros(nC), [-1.0 + DELTA]])
    rup = np.concatenate([np.zeros(nJ), np.full(nC, INF), [INF]])
    fJ = free[J]
    rlo[:nJ][fJ] = 0.0
    fC = free[C]
    rup[nJ:nJ + nC][fC] = 0.0
    lp.row_lower_ = rlo; lp.row_upper_ = rup
    lp.a_matrix_.format_ = highspy.MatrixFormat.kColwise
    lp.a_matrix_.start_ = Ms.indptr.astype(np.int32); lp.a_matrix_.index_ = Ms.indices.astype(np.int32); lp.a_matrix_.value_ = Ms.data.astype(float)
    integ = [highspy.HighsVarType.kContinuous] * nv
    if k:
        for t in range(nR):
            integ[t] = highspy.HighsVarType.kInteger
    for t in range(nR + nV, nv):
        integ[t] = highspy.HighsVarType.kInteger
    lp.integrality_ = integ
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", float(tsep))
    h.setOptionValue("threads", threads)
    h.setOptionValue("mip_improving_solution_save", True)
    h.setOptionValue("mip_feasibility_tolerance", 1e-9)
    h.setOptionValue("primal_feasibility_tolerance", 1e-9)
    # 違反が MINVIOL 未満の解は要らない
    h.setOptionValue("objective_bound", -MINVIOL + 1e-12)
    stp = h.passModel(lp)
    if stp != highspy.HighsStatus.kOk:
        print('passModel', stp, file=sys.stderr)
    t0 = time.time()
    h.run()
    tm = time.time() - t0
    info = {"nR": nR, "nJ": nJ, "nC": nC, "nV": nV, "t": round(tm, 2), "st": h.modelStatusToString(h.getModelStatus()),
            "best": -h.getInfo().objective_function_value if h.getInfo().primal_solution_status else None,
            "dual": -h.getInfo().mip_dual_bound}
    sols = [np.array(s.col_value) for s in h.getSavedMipSolutions()]
    if h.getInfo().primal_solution_status:
        sols.append(np.array(h.getSolution().col_value))
    cuts = []
    for s in sols:
        u = np.zeros(M.T.shape[0])
        uu = s[:nR] * ucoef
        if k:
            uu = np.round(s[:nR]) / k
        u[R] = np.clip(uu, 0.0, None)
        v = np.zeros(M.n)
        v[Cv] = np.clip(s[nR:nR + nV], 0.0, None)
        cut = make_cut(M, u, v, R)
        if cut is None:
            continue
        coef, rhs = cut
        viol = float(coef @ x - rhs)
        if viol < MINVIOL:
            continue
        nz = u[R] > 1e-9
        cuts.append((coef, rhs, {"viol": viol, "nu": int(nz.sum()), "u": [float(t) for t in u[R][nz]]}))
    return cuts, info


def make_cut(M, u, v, R):
    """乗数 u (行)・v (連続列の上限) から CG カットを作り、元の空間の (係数, 右辺) で返す。"""
    ua = M.Tc.T @ u if False else (M.T.T @ u)  # 長さ n
    ua = np.asarray(ua).ravel() + v
    ub = float(M.b @ u + np.dot(v[v > 0], M.W[v > 0]))
    ct = np.zeros(M.n)
    free = M.sig == 0
    for j in np.where(M.isint)[0]:
        a = ua[j]
        if free[j]:
            r = round(a)
            if abs(a - r) > 1e-6:
                return None
            ct[j] = r
        else:
            ct[j] = math.floor(a + 1e-9)
    cont = ~M.isint
    if np.any(ua[cont & ~free] < -1e-7) or np.any(np.abs(ua[cont & free]) > 1e-7):
        return None
    rhs = math.floor(ub + 1e-9)
    # x' = σ (x - d) で戻す
    sg = np.where(M.sig == 0, 1.0, M.sig)
    coef = ct * sg
    rhs_x = rhs + float(np.dot(coef, M.d))
    if not np.any(coef != 0):
        return None
    return coef, rhs_x


def read_sol(path, names):
    pos = {nm: j for j, nm in enumerate(names)}
    x = np.zeros(len(names))
    op = gzip.open if path.endswith(".gz") else open
    with op(path, "rt") as f:
        for l in f:
            p = l.split()
            if len(p) >= 2 and p[0] in pos:
                x[pos[p[0]]] = float(p[1])
    return x


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mps"); ap.add_argument("mode")
    ap.add_argument("--tsep", type=float, default=10.0)
    ap.add_argument("--rounds", type=int, default=30)
    ap.add_argument("--tmax", type=float, default=300.0)
    ap.add_argument("--sol", default=None)
    ap.add_argument("--out", default=None)
    ap.add_argument("--name", default=None)
    ap.add_argument("--threads", type=int, default=1)
    ap.add_argument("--smax", type=float, default=None)
    a = ap.parse_args()
    global SMAX
    if a.smax is not None:
        SMAX = a.smax
    T0 = time.time()
    M = Model(a.mps)
    ks = {"cg": [None], "mod2": [2], "modk": [2, 3, 5, 7]}.get(a.mode)
    if ks is None and a.mode.startswith("modK"):
        ks = [int(a.mode[4:])]
    # LP
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("threads", a.threads)
    h.readModel(a.mps)
    names = list(h.getLp().col_names_)
    h.changeColsIntegrality(M.n, np.arange(M.n, dtype=np.int32), np.array([highspy.HighsVarType.kContinuous] * M.n))
    h.run()
    st = h.modelStatusToString(h.getModelStatus())
    z0 = h.getInfo().objective_function_value
    xs = np.array(h.getSolution().col_value)
    xsol = read_sol(a.sol, names) if a.sol else None
    hist = [{"r": 0, "z": z0, "t": round(time.time() - T0, 2)}]
    ncut, nbad, us = 0, 0, []
    seen = set()
    stop = "rounds"
    for r in range(1, a.rounds + 1):
        rem = a.tmax - (time.time() - T0)
        if rem < 1:
            stop = "tmax"; break
        allc, infos = [], []
        for k in ks:
            cuts, info = separate(M, xs, k, min(a.tsep, max(1.0, rem / len(ks))), a.threads)
            info["k"] = k
            infos.append(info)
            allc += cuts
        added = 0
        for coef, rhs, ci in allc:
            nz = np.nonzero(coef)[0]
            key = (tuple(nz), tuple(np.round(coef[nz], 9)), round(rhs, 9))
            if key in seen:
                continue
            seen.add(key)
            if xsol is not None and float(coef @ xsol) > rhs + 1e-5 * (1.0 + float(np.abs(coef).sum())):  # 既知解は整数性が 1e-5 程度ずれる (binkar10_1 で 7.4e-6)
                nbad += 1
                print(f"BAD cut: viol at sol {float(coef @ xsol) - rhs:.3g} rhs {rhs:.6g} nnz {len(nz)} maxcoef {np.max(np.abs(coef)):.3g} u {ci['u'][:20]}", file=sys.stderr)
            h.addRow(-INF, rhs, len(nz), nz.astype(np.int32), coef[nz])
            us.append(ci["u"])
            added += 1
        ncut += added
        if added == 0:
            hist.append({"r": r, "z": hist[-1]["z"], "t": round(time.time() - T0, 2), "added": 0, "sep": infos})
            stop = "nocut"; break
        h.run()
        if h.getModelStatus() != highspy.HighsModelStatus.kOptimal:
            stop = "lp:" + h.modelStatusToString(h.getModelStatus()); break
        z = h.getInfo().objective_function_value
        xs = np.array(h.getSolution().col_value)
        hist.append({"r": r, "z": z, "t": round(time.time() - T0, 2), "added": added, "sep": infos})
        print(f"round {r}: +{added} z {z:.6g} ({time.time() - T0:.1f}s) {[(i['k'], i.get('st'), i.get('best'), i.get('t')) for i in infos]}", file=sys.stderr, flush=True)
    # u の分母の集計 (各カットの非零の u の分母の最大、limit_denominator(64))
    dens = []
    for uu in us:
        dm = 1
        for t in uu:
            dm = max(dm, Fraction(t).limit_denominator(64).denominator)
        dens.append(dm)
    rec = {"name": a.name or a.mps, "mode": a.mode, "lp_status": st, "lp": z0, "bound": hist[-1]["z"], "rounds": len(hist) - 1,
           "ncut": ncut, "nbad": nbad if xsol is not None else None, "stop": stop, "time": round(time.time() - T0, 1),
           "m": M.T.shape[0], "n": M.n, "nscaled": M.nscaled, "nfail": M.nfail, "npure": int(M.pure.sum()), "sense": M.sense,
           "dens": {str(dd): dens.count(dd) for dd in sorted(set(dens))},
           "hist": [{kk: vv for kk, vv in hh.items()} for hh in hist]}
    line = json.dumps(rec)
    if a.out:
        with open(a.out, "a") as f:
            f.write(line + "\n")
    print(line)


if __name__ == "__main__":
    main()
