"""orbital fixing の正しさ確認用: 頂点推移的なグラフ (巡回グラフ・超立方体) 上の重み付き安定集合・頂点被覆・
支配集合 (オービトープにならない対称性) を HiGHS の最適値と突き合わせる。
python fuzz_of.py 問題数 [開始seed]"""
import random, sys, itertools, highspy
from enomoto_solver import Model, Variable

inf = float('inf')
SCALE = int(sys.argv[3]) if len(sys.argv) > 3 else 1


def circulant(n, jumps):
    e = set()
    for i in range(n):
        for d in jumps:
            a, b = i, (i + d) % n
            if a != b:
                e.add((min(a, b), max(a, b)))
    return n, sorted(e)


def hypercube(k):
    n = 1 << k
    e = set()
    for i in range(n):
        for b in range(k):
            j = i ^ (1 << b)
            e.add((min(i, j), max(i, j)))
    return n, sorted(e)


def gen(seed):
    g = random.Random(seed)
    if g.random() < 0.6:
        n = g.randint(9, 40) * SCALE
        jumps = sorted(set(g.randint(1, n // 2) for _ in range(g.randint(1, 3))))
        n, e = circulant(n, jumps)
    else:
        n, e = hypercube(g.randint(3, 5) + (SCALE > 1) * g.randint(0, 2))
    kind = g.choice(["stable", "cover", "dom", "stable_knap"])
    lo = [0.0] * n
    up = [1.0] * n
    c = [1.0] * n
    rows, rlo, rup = [], [], []
    if kind in ("stable", "stable_knap"):
        c = [-1.0] * n
        for a, b in e:
            rows.append([(a, 1.0), (b, 1.0)]); rlo.append(-inf); rup.append(1.0)
        if kind == "stable_knap":
            # 全頂点に同じ重みの容量制約 (対称性を保つ)
            rows.append([(j, 1.0) for j in range(n)]); rlo.append(-inf); rup.append(float(g.randint(2, max(2, n // 3))))
    elif kind == "cover":
        for a, b in e:
            rows.append([(a, 1.0), (b, 1.0)]); rlo.append(1.0); rup.append(inf)
    else:
        adj = [[j] for j in range(n)]
        for a, b in e:
            adj[a].append(b); adj[b].append(a)
        for j in range(n):
            rows.append([(k, 1.0) for k in adj[j]]); rlo.append(1.0); rup.append(inf)
    # 一部の問題では連続列 (各頂点の余裕) を足す
    return lo, up, c, rows, rlo, rup, kind, n


def solve_highs(lo, up, c, rows, rlo, rup):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 0.0)
    nv = len(lo)
    for j in range(nv):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
    h.changeColsIntegrality(nv, list(range(nv)), [highspy.HighsVarType.kInteger] * nv)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l, u, len(r), [j for j, _ in r], [a for _, a in r])
    h.run()
    st = h.getModelStatus()
    if st != highspy.HighsModelStatus.kOptimal:
        return None
    return h.getInfo().objective_function_value


def solve_ours(lo, up, c, rows, rlo, rup):
    M = Model(); xs = [Variable(int, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9, log=False)
    return s.objective if s.status == "optimal" else None


if __name__ == '__main__':
    cnt = int(sys.argv[1]); s0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0
    for seed in range(s0, s0 + cnt):
        lo, up, c, rows, rlo, rup, kind, n = gen(seed)
        ref = solve_highs(lo, up, c, rows, rlo, rup)
        ours = solve_ours(lo, up, c, rows, rlo, rup)
        ok = (ref is None and ours is None) or (ref is not None and ours is not None and abs(ref - ours) <= 1e-6 * max(1, abs(ref)))
        if not ok:
            bad += 1
            print(f"BAD seed {seed} kind {kind} n {n}: highs {ref} ours {ours}", flush=True)
    print(f"bad {bad} of {cnt}")
