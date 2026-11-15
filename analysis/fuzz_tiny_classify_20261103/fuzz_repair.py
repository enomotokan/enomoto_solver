"""後処理の修復 (`ENOMOTO_MIP_POSTSOLVE_REPAIR`) が起きる小さな MIP を探す乱数の生成。
連続列 s を小さい係数 eps で持つ等式 `eps s + sum a x + y = b` (代入で s を消すと行の許容誤差が 1/eps 倍に広がる) と 2 値のナップサック。
使い方: ENOMOTO_MIP_LOG=1 python fuzz_repair.py 問題数 [開始seed] 2>&1 | grep 'postsolve repair'  (seed ごとに 'SEED n' を出す)"""
import random, sys
from enomoto_solver import Model, Variable
inf = float("inf")
def gen(seed):
    g = random.Random(seed); nb = g.randint(3, 8); ns = g.randint(1, 4); ny = g.randint(1, 4)
    lo = [0.0] * nb + [0.0] * ns + [0.0] * ny; up = [1.0] * nb + [g.choice([10.0, 100.0, 1e4]) for _ in range(ns)] + [float(g.randint(1, 5)) for _ in range(ny)]
    it = [True] * nb + [False] * (ns + ny)
    c = [-round(g.random() * 10 + 1, 2) for _ in range(nb)] + [round(g.random() * 2, 3) for _ in range(ns)] + [round(g.random() * 3 - 1, 2) for _ in range(ny)]
    rows = []; rlo = []; rup = []
    for k in range(ns):
        s = nb + k; eps = g.choice([1e-3, 1e-4, 1e-5, 1e-6]); xs = g.sample(range(nb), g.randint(1, min(3, nb))); y = nb + ns + g.randrange(ny)
        r = [(s, eps)] + [(j, float(g.randint(1, 3))) for j in xs] + [(y, 1.0)]
        b = round(g.random() * 4, 3); rows.append(r); rlo.append(b); rup.append(b)
    for _ in range(g.randint(1, 3)):
        xs = g.sample(range(nb), g.randint(2, nb)); rows.append([(j, float(g.randint(1, 9))) for j in xs]); rlo.append(-inf); rup.append(float(g.randint(5, 20)))
    return lo, up, c, it, rows, rlo, rup
def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective
if __name__ == "__main__":
    N = int(sys.argv[1]); st = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    for seed in range(st, st + N):
        print("SEED", seed, flush=True); print("RES", seed, *ours(gen(seed)), flush=True)
