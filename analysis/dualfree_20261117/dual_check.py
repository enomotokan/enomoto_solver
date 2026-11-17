"""確かめ (a): 規則で等式に上げた LP (P') と元の LP (P) を HiGHS で解き、次を確かめる (python dual_check.py 問題数 [開始seed])。
1. 最適値が等しい (状態も同じ)。
2. P' の最適解は P で実行可能で、上げた行のスラックは 0 (等式)。
3. 根拠の列の境界を外した P'' (境界は陰に導けるので P' と同じ問題) の最適な双対で、上げた行の双対は ≦ 行の符号
   (y ≤ 0、HiGHS の符号) を満たす (= P の双対としても実行可能)。境界を外さない P' では、根拠の列が導ける値ちょうどの
   境界にいる退化の場合に、被約費用が 0 でない別の最適な双対 (y > 0) が返ることがある (最適値は同じ)。
4. P' の最適解は P でも最適 (目的値が P の最適値に等しい)。
規則は src/presolve/dualfreeineq.rs と同じものを Python で書いた (選ぶ行も同じ順)。"""
import sys, time, highspy
sys.path.insert(0, __file__.rsplit('/', 1)[0])
from gen import gen
inf = float('inf'); TOL = 1e-9

def normalize(lp):
    """rows を ≦ 行 (G) と等式 (A) に分ける。戻り値の G は (行, 右辺, 元の行番号, 符号)"""
    lo, up, c, it, rows, rlo, rup = lp
    A = []; G = []
    for i, (r, l, u) in enumerate(zip(rows, rlo, rup)):
        if l == u: A.append((r, l, i)); continue
        if u < inf: G.append((r, u, i, 1.0))
        if l > -inf: G.append(([(j, -v) for j, v in r], -l, i, -1.0))
    return A, G

def rule(n, A, G, c, lo, up, is_int):
    in_eq = [False] * n
    for r, _, _ in A:
        for j, _ in r: in_eq[j] = True
    col = [[] for _ in range(n)]
    for i, (r, _, _, _) in enumerate(G):
        for j, v in r:
            if v != 0: col[j].append((i, v))
    act = []; ninf = []
    for r, _, _, _ in G:
        s = 0.0; k = 0
        for j, v in r:
            b = lo[j] if v > 0 else up[j]
            if b in (inf, -inf): k += 1
            else: s += v * b
        act.append(s); ninf.append(k)
    def resid(i, j, v):
        b = lo[j] if v > 0 else up[j]
        if b not in (inf, -inf): return act[i] - v * b if ninf[i] == 0 else -inf
        return act[i] if ninf[i] == 1 else -inf
    used = [False] * len(G); prom = [False] * len(G); out = []
    for j in range(n):
        if in_eq[j] or not col[j] or is_int[j] or lo[j] >= up[j]: continue
        L, U = -inf, inf
        for i, v in col[j]:
            r = resid(i, j, v)
            if r == -inf: continue
            b = (G[i][1] - r) / v
            if v > 0: U = min(U, b)
            else: L = max(L, b)
        if not ((lo[j] == -inf or L >= lo[j] - TOL) and (up[j] == inf or U <= up[j] + TOL)): continue
        if any(prom[i] for i, _ in col[j]): continue
        npos = sum(1 for _, v in col[j] if v > 0); nneg = len(col[j]) - npos
        if npos == 1 and c[j] <= 0: r = next(i for i, v in col[j] if v > 0)
        elif nneg == 1 and c[j] >= 0: r = next(i for i, v in col[j] if v < 0)
        else: continue
        if used[r]: continue
        prom[r] = True; out.append((r, j))
        for i, _ in col[j]:
            if i != r: used[i] = True
    return out

def solve(n, lo, up, c, A, G, eqset):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); K = highspy.kHighsInf
    for j in range(n):
        h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
    for r, b, _ in A: h.addRow(b, b, len(r), [j for j, _ in r], [v for _, v in r])
    for i, (r, b, _, _) in enumerate(G):
        h.addRow(b if i in eqset else -K, b, len(r), [j for j, _ in r], [v for _, v in r])
    h.run()
    st = h.modelStatusToString(h.getModelStatus())
    s = h.getSolution()
    return st, h.getInfo().objective_function_value, list(s.col_value), list(s.row_dual)

N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; fired = 0; npro = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed, lp_only=True)
    lo, up, c, it, rows, rlo, rup = lp; n = len(lo)
    A, G = normalize(lp)
    pw = rule(n, A, G, c, lo, up, [False] * n)
    pro = [r for r, _ in pw]
    # 根拠の列の境界を外した P'' (境界は陰に導けるので同じ問題)。双対の符号はこちらで確かめる
    lo2 = lo[:]; up2 = up[:]
    for _, j in pw: lo2[j] = -inf; up2[j] = inf
    if not pro: continue
    fired += 1; npro += len(pro)
    s0, o0, _, _ = solve(n, lo, up, c, A, G, set())
    s1, o1, x1, _ = solve(n, lo, up, c, A, G, set(pro))
    s2, o2, _, y1 = solve(n, lo2, up2, c, A, G, set(pro))
    msg = []
    if s0 != s1 or s0 != s2: msg.append(f"状態 {s0} / {s1} / {s2}")
    elif s0 == 'Optimal':
        if abs(o0 - o1) > 1e-6 * (1 + abs(o0)) or abs(o0 - o2) > 1e-6 * (1 + abs(o0)): msg.append(f"最適値 {o0} / {o1} / {o2}")
        # P での実行可能性 (相対 1e-7)
        for j in range(n):
            if x1[j] < lo[j] - 1e-7 * (1 + abs(lo[j])) or x1[j] > up[j] + 1e-7 * (1 + abs(up[j])): msg.append(f"列 {j} の境界"); break
        for r, l, u in zip(rows, rlo, rup):
            a = sum(v * x1[j] for j, v in r); sc = 1e-7 * (1 + sum(abs(v * x1[j]) for j, v in r))
            if a < l - sc or a > u + sc: msg.append("行の違反"); break
        for i in pro:
            r, b, _, _ = G[i]
            a = sum(v * x1[j] for j, v in r)
            if abs(a - b) > 1e-7 * (1 + abs(b)): msg.append(f"上げた行 {i} のスラック {b - a}")
            # 行の双対: A の行が先に並ぶ
            y = y1[len(A) + i]
            if y > 1e-7 * (1 + max(abs(v) for v in c)): msg.append(f"上げた行 {i} の双対 {y} > 0")
    if msg:
        bad += 1
        if bad <= 15: print("seed", seed, "; ".join(msg[:4]))
print(f"bad {bad} / 規則が当たった {fired} 問 (上げた行 {npro}) / 全 {N} 問, time {time.time() - t0:.1f}")
