"""OBJINT の誤答の検査: 目的値の刻みが暗黙の整数 (行の規則・HiGHS の双対側の規則) や固定の列から付く形の小さな MIP を乱数で作り、
HiGHS (mip_rel_gap 1e-9) と最適値を突き合わせる。形: メイクスパン (連続列 c >= Σ a x + b を数本)、連続列どうしの差の行 (連鎖)、
等式 y = Σ a x + b、ナップサック、費用のある固定の列 (整数値・端数)、目的値の定数、最大化、費用の刻み (1・0.5・1/3・2・0.1・7)。
確率で刻みを壊す要素 (端数の右辺・端数の係数・端数の境界) を混ぜる。整数列は 8〜22 本で、木の中の刈り込みが効く大きさにする。
判定: 状態が同じで最適値が相対 1e-6 で一致、ENOMOTO の点が元の問題で許容誤差 1e-6 で実行可能で目的値が一致。
使い方: ENOMOTO_MIP_OBJINT=1 python fuzz_objint.py 問題数 [開始seed]   (付けなければ既定の動きの検査)。1 スレッドで解く。"""
import os, random, re, sys, tempfile, time
NODES = []
LASTLOG = [""]
NEWSTEP = []
LATW = []
os.environ.setdefault("ENOMOTO_MIP_THREADS", "1")
os.environ.setdefault("ENOMOTO_MIP_LOG", "1")
import highspy
from enomoto_solver import Model, Variable
inf = float("inf")


def gen(seed):
    g = random.Random(seed)
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    brk = g.random() < 0.4
    frac = lambda: brk and g.random() < 0.25
    def var(l, u, integer):
        lo.append(l); up.append(u); c.append(0.0); it.append(integer); return len(lo) - 1
    def row(r, l, u):
        rows.append(r); rlo.append(l); rup.append(u)
    if g.random() < 0.6:
        return gen_pm(g, brk, frac)
    nx = g.randint(12, 40)
    X = [var(0.0, float(g.choice([1, 1, 1, 3, 5])), True) for _ in range(nx)]
    ny = g.randint(1, 4)
    Y = [var(0.0 if g.random() < 0.8 else float(g.randint(-5, 5)) + (0.5 if frac() else 0.0), inf if g.random() < 0.7 else float(g.randint(30, 80)), False) for _ in range(ny)]
    coef = lambda: float(g.randint(1, 9)) + (0.5 if frac() else 0.0)
    # メイクスパン: y_k >= Σ a x + b (選んだ整数列の部分集合ごと)
    for k in Y:
        for _ in range(g.randint(1, 5)):
            S = g.sample(X, g.randint(2, min(6, nx)))
            row([(k, 1.0)] + [(j, -coef()) for j in S], float(g.randint(-3, 3)) + (0.5 if frac() else 0.0), inf)
    # 連続列どうしの差 (連鎖): y_k - y_l + Σ a x >= b
    for _ in range(g.randint(0, 2)):
        if ny >= 2:
            a, b = g.sample(Y, 2)
            S = g.sample(X, g.randint(1, 3))
            row([(a, 1.0), (b, -1.0)] + [(j, -coef()) for j in S], float(g.randint(-2, 2)), inf)
    # 等式で決まる連続列 z = Σ a x + b
    Z = []
    for _ in range(g.randint(0, 2)):
        z = var(-inf, inf, False); Z.append(z)
        S = g.sample(X, g.randint(1, 4))
        row([(z, 1.0)] + [(j, -coef()) for j in S], float(g.randint(-4, 4)) + (0.5 if frac() else 0.0), float(g.randint(-4, 4)) if False else None)
        rup[-1] = rlo[-1]
    # 割り当て・ナップサックで整数列を縛る
    for _ in range(g.randint(2, 6)):
        S = g.sample(X, g.randint(2, min(8, nx)))
        if g.random() < 0.5:
            row([(j, float(g.randint(1, 9))) for j in S], float(g.randint(3, 12)), inf)
        else:
            row([(j, float(g.randint(1, 9))) for j in S], -inf, float(g.randint(8, 30)))
    # 目的: 刻み s の整数倍
    s = g.choice([1.0, 1.0, 0.5, 1.0 / 3.0, 2.0, 0.1, 7.0])
    for k in Y + Z:
        c[k] = s * g.choice([1, 1, 2, 3] + ([-1] if up[k] < inf else [])) if g.random() < 0.85 else 0.0
    for j in X:
        if g.random() < 0.5:
            c[j] = s * g.randint(-6, 6)
    if frac():
        c[g.choice(Y)] += 0.37 * s
    # 費用のある固定の列 (行に出ない)
    if g.random() < 0.3:
        v = float(g.randint(-3, 3)) + (0.25 if frac() else 0.0)
        f = var(v, v, False); c[f] = s * g.randint(1, 4)
    # 費用のある、どこにも出ない連続列 (有界)
    if g.random() < 0.2:
        f = var(float(g.randint(0, 2)), float(g.randint(3, 5)) + (0.5 if frac() else 0.0), False); c[f] = s * g.choice([1, -1])
    add_cont_part(g, s, var, row, c, it, lo, up)
    off = g.choice([0.0, 0.0, 0.3, -2.0, 5.5])
    sense = g.choice(["minimize", "minimize", "maximize"])
    if sense == "maximize":
        # 最大化でも有界になるように、費用の符号を返す (Y・Z は下に有界なので最大化の向きでは費用を負にする)
        c = [-v for v in c]
    return (lo, up, c, it, rows, rlo, rup, off, sense)


def add_cont_part(g, s, var, row, c, it, lo, up):
    """OBJINT の `c` の形: 費用の小さい有界な連続列 w (Σ |c| (u - l) が刻みより小さいことが多い) を 1〜3 本と、整数でない値に固定された
    費用のある列を確率で足す。w は整数列と行でつなぐ (w <= Σ a x + b、w >= Σ a x - b)"""
    X = [j for j in range(len(lo)) if it[j]]
    if g.random() < 0.6 and X:
        for _ in range(g.randint(1, 3)):
            u = float(g.randint(1, 10)) + (0.3 if g.random() < 0.7 else 0.0)  # 端数の境界なら HO2 の印が付かず連続部に残る
            w = var(0.0, u, False)
            c[w] = s * g.choice([0.01, 0.02, -0.01, 0.03, 0.05, -0.05, 0.3])
            S = g.sample(X, min(len(X), g.randint(1, 3)))
            if g.random() < 0.5:
                row([(w, 1.0)] + [(j, -float(g.randint(1, 4))) for j in S], -float("inf"), float(g.randint(0, 3)) + 0.5 * g.randint(0, 1))
            else:
                row([(w, 1.0)] + [(j, -float(g.randint(1, 4))) for j in S], -float(g.randint(0, 6)) - 0.25 * g.randint(0, 1), float("inf"))
    if g.random() < 0.3:
        v = round(g.uniform(-3, 3), 2)
        f = var(v, v, False); c[f] = s * g.choice([1, 0.37, -2])


def gen_pm(g, brk, frac):
    """並列機械の割り当て (x_jm 二値、Σ_m x_jm = 1) とメイクスパン C >= Σ_j p_j x_jm (+ 機械ごとの準備時間)。LP の下界が弱く分枝が要る。
    目的: s (k C + Σ 割り当ての費用) + 定数。確率で端数の処理時間・端数の準備時間・機械の開始時刻の連続列 (連鎖) を混ぜる"""
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    def var(l, u, integer):
        lo.append(l); up.append(u); c.append(0.0); it.append(integer); return len(lo) - 1
    def row(r, l, u):
        rows.append(r); rlo.append(l); rup.append(u)
    J, M = g.randint(7, 13), g.randint(2, 4)
    p = [float(g.randint(3, 20)) + (0.5 if frac() else 0.0) for _ in range(J)]
    x = [[var(0.0, 1.0, True) for _ in range(M)] for _ in range(J)]
    C = var(0.0, inf, False)
    for j in range(J):
        row([(x[j][m], 1.0) for m in range(M)], 1.0, 1.0)
    for m in range(M):
        r = [(x[j][m], -p[j] * (1.0 + (0.5 * g.randint(0, 1) if g.random() < 0.3 else 0.0))) for j in range(J)]
        if g.random() < 0.3:
            # 機械の開始時刻 S_m (連続) と C >= S_m + Σ p x
            S = var(0.0, inf, False)
            row([(S, 1.0)], float(g.randint(0, 4)) + (0.5 if frac() else 0.0), inf)
            row([(C, 1.0), (S, -1.0)] + r, 0.0, inf)
        else:
            row([(C, 1.0)] + r, float(g.randint(0, 3)) + (0.5 if frac() else 0.0), inf)
    s = g.choice([1.0, 1.0, 0.5, 1.0 / 3.0, 2.0, 0.1, 7.0])
    c[C] = s * g.randint(1, 3)
    for j in range(J):
        for m in range(M):
            if g.random() < 0.3:
                c[x[j][m]] = s * g.randint(0, 4)
    if frac():
        c[C] += 0.37 * s
    if g.random() < 0.3:
        v = float(g.randint(-3, 3)) + (0.25 if frac() else 0.0)
        f = var(v, v, False); c[f] = s * g.randint(1, 4)
    add_cont_part(g, s, var, row, c, it, lo, up)
    off = g.choice([0.0, 0.0, 0.3, -2.0, 5.5])
    sense = g.choice(["minimize", "minimize", "maximize"])
    if sense == "maximize":
        c = [-v for v in c]
    return (lo, up, c, it, rows, rlo, rup, off, sense)


def highs(lp):
    lo, up, c, it, rows, rlo, rup, off, sense = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9); h.setOptionValue("threads", 1)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.changeObjectiveOffset(off)
    h.changeObjectiveSense(highspy.ObjSense.kMaximize if sense == "maximize" else highspy.ObjSense.kMinimize)
    h.run()
    return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value


def ours(lp):
    lo, up, c, it, rows, rlo, rup, off, sense = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)) + off, sense)
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    sys.stdout.flush()
    fd, path = tempfile.mkstemp(); saved = os.dup(2); saved1 = os.dup(1); os.dup2(fd, 2); os.dup2(fd, 1)
    try:
        s = M.solve(mip_rel_gap=1e-9)
    finally:
        sys.stdout.flush(); os.dup2(saved, 2); os.dup2(saved1, 1); os.close(saved); os.close(saved1); os.close(fd)
    log = open(path).read(); os.unlink(path); LASTLOG[0] = log
    m = re.findall(r"nodes (\d+) \(", log)
    NODES.append(int(m[-1]) if m else -1)
    # OBJINT で刻みが新しく付いたか (既定の判定では付かない)
    st = re.findall(r"objective step with implied integers: (Some\([^)]*\)|None) \(without: (Some\([^)]*\)|None)\)", log)
    NEWSTEP.append(any(a != "None" and b == "None" for a, b in st))
    # `c`: 連続部に幅のある範囲が付いたか (m < M)
    lat = re.findall(r"OBJINT c: lattice Some\(ObjLat \{ s: ([-0-9.e]+), base: ([-0-9.e]+), m: ([-0-9.e]+), mm: ([-0-9.e]+) \}\)", log)
    LATW.append(any(float(mm) - float(m) > 1e-6 for _, _, m, mm in lat))
    return s.status, s.objective, ([x.value for x in xs] if s.status == "optimal" else None)


def feasible(lp, x, tol=1e-6):
    lo, up, c, it, rows, rlo, rup = lp[:7]
    for j in range(len(lo)):
        if x[j] < lo[j] - tol or x[j] > up[j] + tol or (it[j] and abs(x[j] - round(x[j])) > tol): return False
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - tol * (1 + abs(l)) or a > u + tol * (1 + abs(u)): return False
    return True


if __name__ == "__main__":
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; t0 = time.time(); st = {}
    for seed in range(start, start + N):
        lp = gen(seed)
        hs, ho = highs(lp); os_, oo, x = ours(lp)
        hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded", "Primal infeasible or unbounded": "unbounded"}.get(hs, hs)
        ok = (hmap == os_ or {hmap, os_} <= {"unbounded", "infeasible_or_unbounded", "infeasible"}) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if ok and x is not None:
            ok = feasible(lp, x) and abs(sum(cc * v for cc, v in zip(lp[2], x)) + lp[7] - oo) <= 1e-6 * (1 + abs(oo))
        st[os_] = st.get(os_, 0) + 1
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
    print("bad", bad, "of", N, "status", st, "with nodes>0", sum(n > 0 for n in NODES), "new step", sum(NEWSTEP), "new step and nodes>0", sum(a and n > 0 for a, n in zip(NEWSTEP, NODES)), "c range with width", sum(LATW), "c and nodes>0", sum(a and n > 0 for a, n in zip(LATW, NODES)), "time %.1f" % (time.time() - t0))
