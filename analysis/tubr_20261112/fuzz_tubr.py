"""TUBR (分枝しなくてよい整数列、analysis/tubr_20261112.md) の誤答の検査: ネットワーク行列の形の整数計画の写し (drcrtu_20261111/fuzz_net.py の
作り方) と、施設の開設 y (0-1) と整数の流量 x (x_ij ≤ u_j y_j、需要の等式) の問題を、ENOMOTO_MIP_TUBR を付けて解き HiGHS と突き合わせる。
ENOMOTO_MIP_LOG=1 を付けると stderr の "MIP: TUBR" の行から、I' のある問題・繊維の LP で解けたノードを数える。

python analysis/tubr_20261112/fuzz_tubr.py 問題数 [開始seed]
畳みの切り替えはこのプロセスの環境変数 ENOMOTO_MIP_FOLD で選ぶ (例 rin)。ENOMOTO_MIP_LOG=1 を付けると stderr の MIP_FOLD の行から
畳んだ問題・整数の組を認めた数・安全網に落ちた数を数える。

作り方: 乱数の有向の木 (弧 = 行) と道 (列) でネットワーク行列 N (nr x nv、±1) を作り、整数の変数 x (0..u) で lo ≤ N x ≤ up の
行にする。これを k 個写し (写しごとに確率で反転 x = u − x')、写しの間を結ぶ行 (同じ列の写しの和、または写しをまたぐ道) を足す。
一部は N の転置を使う、一部は N の 1 つの係数の符号を変えて TU を崩す、一部は 1 つの係数を変えて対称性を崩す。
"""
import os, random, sys, tempfile, time
sys.argv_saved = list(sys.argv)
from pathlib import Path
D = Path(__file__).resolve().parent
inf = float('inf')


def net_matrix(g, nr, nv):
    n = nr + 1
    par = [0] * n; upw = [True] * n; depth = [0] * n
    for v in range(1, n):
        par[v] = v - 1 if g.random() < 0.4 else g.randrange(v)
        upw[v] = g.random() < 0.5; depth[v] = depth[par[v]] + 1
    cols = []
    while len(cols) < nv:
        a, b = g.randrange(n), g.randrange(n)
        if a == b: continue
        col = {}
        while a != b:
            if depth[a] >= depth[b]: col[a - 1] = 1.0 if upw[a] else -1.0; a = par[a]
            else: col[b - 1] = -1.0 if upw[b] else 1.0; b = par[b]
        cols.append(col)
    return cols  # 列ごとに {行: 係数}


def gen(seed):
    g = random.Random(seed)
    nr = g.randint(2, 5); nv = g.randint(2, 5); k = g.randint(2, 4)
    cols = net_matrix(g, nr, nv)
    if g.random() < 0.3:  # 転置
        t = [dict() for _ in range(nr)]
        for j, c in enumerate(cols):
            for i, a in c.items(): t[i][j] = a
        cols = [c for c in t if c]; nr, nv = nv, len(cols)
    if g.random() < 0.2:  # TU を崩す
        j = g.randrange(nv); i = g.choice(list(cols[j])); cols[j][i] = -cols[j][i]
    up0 = [float(g.randint(1, 3)) for _ in range(nv)]
    c0 = [float(g.randint(-5, 5)) for _ in range(nv)]
    rows0 = []
    for i in range(nr):
        r = [(j, c[i]) for j, c in enumerate(cols) if i in c]
        if not r: continue
        act = sum(a * up0[j] / 2 for j, a in r)
        s = g.choice(['le', 'ge', 'eq', 'rng'])
        rows0.append((r, s, float(round(act + g.randint(-1, 1)))))
    flip = [g.random() < 0.3 for _ in range(k)]
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    vid = {}
    for cc in range(k):
        for j in range(nv):
            vid[cc, j] = len(lo); lo.append(0.0); up.append(up0[j]); it.append(True)
            c.append(-c0[j] if flip[cc] else c0[j])
    def add(terms, sense, rhs):
        r = []; b = rhs
        for (cc, j), a in terms:
            if flip[cc]: r.append((vid[cc, j], -a)); b -= a * up0[j]
            else: r.append((vid[cc, j], a))
        l, u = {'le': (-inf, b), 'ge': (b, inf), 'eq': (b, b), 'rng': (b - 1, b + 1)}[sense]
        rows.append(r); rlo.append(l); rup.append(u)
    for cc in range(k):
        for r, s, b in rows0:
            add([((cc, j), a) for j, a in r], s, b)
    for j in range(nv):
        if g.random() < 0.7:
            tot = sum(up0[j] for _ in range(k))
            add([((cc, j), 1.0) for cc in range(k)], g.choice(['le', 'ge', 'eq']), float(round(tot * g.uniform(0.2, 0.8))))
    if g.random() < 0.3 and nv >= 2:  # 写しをまたぐ行 (対称に、写しを回して)
        j1, j2 = g.sample(range(nv), 2)
        for cc in range(k):
            add([((cc, j1), 1.0), (((cc + 1) % k, j2), -1.0)], 'le', 1.0)
    if g.random() < 0.15 and rows:
        r = g.randrange(len(rows)); i = g.randrange(len(rows[r])); rows[r][i] = (rows[r][i][0], rows[r][i][1] + 1.0)
    n = len(lo); perm = list(range(n)); g.shuffle(perm); inv = {p: i for i, p in enumerate(perm)}
    lo = [lo[p] for p in perm]; up = [up[p] for p in perm]; c = [c[p] for p in perm]; it = [it[p] for p in perm]
    rows = [[(inv[j], a) for j, a in r] for r in rows]
    rp = list(range(len(rows))); g.shuffle(rp)
    rows = [rows[i] for i in rp]; rlo = [rlo[i] for i in rp]; rup = [rup[i] for i in rp]
    return lo, up, c, it, rows, rlo, rup




def gen_facility(seed):
    g = random.Random(seed)
    nf = g.randint(2, 4); nc = g.randint(2, 5)
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    y = []
    for f in range(nf):
        y.append(len(lo)); lo.append(0.0); up.append(1.0); c.append(float(g.randint(3, 12))); it.append(True)
    x = {}
    for f in range(nf):
        for k in range(nc):
            x[f, k] = len(lo); lo.append(0.0); up.append(float(g.randint(2, 6))); c.append(float(g.randint(1, 6))); it.append(g.random() < 0.8)
    cap = [float(g.randint(2, 8)) for _ in range(nf)]
    for k in range(nc):
        rows.append([(x[f, k], 1.0) for f in range(nf)]); d = float(g.randint(1, 4)); rlo.append(d); rup.append(d if g.random() < 0.7 else inf)
    for f in range(nf):
        rows.append([(x[f, k], 1.0) for k in range(nc)] + [(y[f], -cap[f])]); rlo.append(-inf); rup.append(0.0)
    if g.random() < 0.3:  # TU を崩す行 (x の 3 つの和)
        ks = g.sample(sorted(x.values()), 3); rows.append([(j, 1.0) for j in ks]); rlo.append(-inf); rup.append(float(g.randint(2, 5)))
    if g.random() < 0.2:  # 端数の右辺
        rows.append([(x[0, 0], 1.0), (x[nf - 1, nc - 1], 1.0)]); rlo.append(-inf); rup.append(4.5)
    return lo, up, c, it, rows, rlo, rup


# fuzz_fold.py の highs・ours・feasible を読み込む (その本体の実行の部分は読まない)
src = (D.parent / "drcr_20261110" / "fuzz_fold.py").read_text()
src = src[: src.index("\nN = int(sys.argv[1])")]
ns = {"__name__": "fuzz_fold"}
sys.argv = [sys.argv[0], "0", "0", "mip"]
exec(compile(src, "fuzz_fold.py", "exec"), ns)
sys.argv = sys.argv_saved
highs, ours, feasible = ns["highs"], ns["ours"], ns["feasible"]


def ours_logged(lp):
    """stderr (fd 2) を一時ファイルに向けて解き、MIP_FOLD の行を返す"""
    f = tempfile.TemporaryFile(mode="w+b"); old = os.dup(2); os.dup2(f.fileno(), 2)
    try:
        r = ours(lp)
    finally:
        os.dup2(old, 2); os.close(old)
    f.seek(0); lines = [l for l in f.read().decode(errors="replace").splitlines() if l.startswith("MIP_FOLD") or "TUBR" in l]
    return r, lines


N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); folded = intg = safety = acc = 0; chk = [0, 0]
for seed in range(start, start + N):
    lp = gen(seed) if seed % 2 else gen_facility(seed)
    hs, ho = highs(lp); (os_, oo, x), lines = ours_logged(lp)
    if any("need no branching" in l and not l.split("MIP: TUBR: ")[1].startswith("0 ") and "not used" not in l for l in lines): folded += 1
    intg += sum(int(l.split("solved by the fiber LP ")[1].split(",")[0]) for l in lines if "solved by the fiber LP" in l)
    for l in lines:
        if "check: integral" in l:
            chk[0] += int(l.split("check: integral ")[1].split()[0]); chk[1] += int(l.split(" fractional ")[-1].split()[0])
    if any("safety net" in l for l in lines): safety += 1
    for l in lines:
        if "accepted" in l: acc += int(l.split("integer groups: ")[1].split(" ")[0])
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    same_inf = hmap in ('infeasible', 'unbounded') and os_ in ('infeasible', 'unbounded', 'infeasible_or_unbounded')
    ok = same_inf or ((hmap == os_) and (hmap != 'optimal' or (abs(ho - oo) <= 1e-6 * (1 + abs(ho)) and feasible(lp, x))))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, lines[-2:], flush=True)
print("ENOMOTO_MIP_TUBR=%s bad %d of %d time %.1f with_iprime=%d fiber_solved_nodes=%d check_integral=%d check_fractional=%d"
      % (os.environ.get("ENOMOTO_MIP_TUBR", "(なし)"), bad, N, time.time() - t0, folded, intg, chk[0], chk[1]))
