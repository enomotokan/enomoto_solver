"""PGO (profile-guided optimization) の学習用に、決まった種から作る小さな整数最適化・線形最適化の問題を解く。

配る wheel の作り方 (scripts/build_dist_wheel.sh、analysis/buildopt_20261122) で、`-Cprofile-generate` で作った
`enomoto_solver` を入れた Python から呼ぶ。問題は MIPLIB などの計測用の問題と重ならないよう、ここで乱数から作る
(ファイルは要らない)。同じ引数なら毎回同じ問題になる。

    python scripts/pgo_train.py [--time-limit 秒] [--scale 倍率] [--threads 1,4]

種類: 多次元ナップサック、集合被覆、容量つき施設配置、一般化割当、段取りつきロットサイズ、一般整数の生産計画、
輸送問題 (LP)、実行不能な問題。どれも上限時間で打ち切る (解き切らなくてよい。木の探索・切除平面・
ヒューリスティック・前処理・LP の各所を通すのが目的)。
"""
import argparse
import math
import random
import time

from enomoto_solver import Model, Variable


def lin(cs, xs):
    f = 0
    for c, x in zip(cs, xs):
        if c:
            f = f + c * x
    return f


def mknap(rng, n=60, m=8):
    M = Model()
    x = [Variable(int, 0, 1) for _ in range(n)]
    for _ in range(m):
        w = [rng.randint(5, 60) for _ in range(n)]
        M.add_constraint(lin(w, x) <= sum(w) // 3)
    M.set_objective(lin([rng.randint(10, 100) for _ in range(n)], x), sense="maximize")
    return M


def setcover(rng, n=200, m=150):
    M = Model()
    x = [Variable(int, 0, 1) for _ in range(n)]
    for _ in range(m):
        cols = rng.sample(range(n), rng.randint(2, 5))
        M.add_constraint(lin([1] * len(cols), [x[j] for j in cols]) >= 1)
    M.set_objective(lin([rng.randint(20, 40) for _ in range(n)], x))
    return M


def facility(rng, nf=12, nc=40):
    M = Model()
    y = [Variable(int, 0, 1) for _ in range(nf)]
    x = [[Variable(float, 0, 1) for _ in range(nf)] for _ in range(nc)]
    cap = [rng.randint(30, 80) for _ in range(nf)]
    dem = [rng.randint(5, 20) for _ in range(nc)]
    for i in range(nc):
        M.add_constraint(lin([1] * nf, x[i]) == 1)
        for j in range(nf):
            M.add_constraint(x[i][j] - y[j] <= 0)
    for j in range(nf):
        M.add_constraint(lin(dem, [x[i][j] for i in range(nc)]) - cap[j] * y[j] <= 0)
    obj = lin([rng.randint(100, 300) for _ in range(nf)], y)
    for i in range(nc):
        obj = obj + lin([dem[i] * rng.randint(1, 30) for _ in range(nf)], x[i])
    M.set_objective(obj)
    return M


def gap(rng, na=8, nt=40):
    M = Model()
    x = [[Variable(int, 0, 1) for _ in range(na)] for _ in range(nt)]
    w = [[rng.randint(5, 25) for _ in range(na)] for _ in range(nt)]
    for t in range(nt):
        M.add_constraint(lin([1] * na, x[t]) == 1)
    for a in range(na):
        M.add_constraint(lin([w[t][a] for t in range(nt)], [x[t][a] for t in range(nt)]) <= int(0.8 * sum(w[t][a] for t in range(nt)) / na) + 10)
    obj = 0
    for t in range(nt):
        obj = obj + lin([rng.randint(10, 50) for _ in range(na)], x[t])
    M.set_objective(obj)
    return M


def lotsizing(rng, ni=5, nt=15):
    M = Model()
    obj = 0
    capt = [ni * rng.randint(18, 30) for _ in range(nt)]
    use = [[None] * nt for _ in range(ni)]
    for i in range(ni):
        d = [rng.randint(0, 30) for _ in range(nt)]
        big = sum(d)
        prev = 0
        for t in range(nt):
            q = Variable(float, 0, big)
            s = Variable(float, 0, big)
            z = Variable(int, 0, 1)
            M.add_constraint(prev + q - s == d[t])
            M.add_constraint(q - big * z <= 0)
            use[i][t] = q
            obj = obj + rng.randint(1, 3) * s + rng.randint(50, 150) * z
            prev = s
    for t in range(nt):
        M.add_constraint(lin([1] * ni, [use[i][t] for i in range(ni)]) <= capt[t])
    M.set_objective(obj)
    return M


def genint(rng, n=30, m=15):
    M = Model()
    x = [Variable(int, 0, rng.randint(5, 40)) for _ in range(n)]
    for _ in range(m):
        cols = rng.sample(range(n), rng.randint(4, 12))
        a = [rng.randint(-5, 15) for _ in cols]
        M.add_constraint(lin(a, [x[j] for j in cols]) <= rng.randint(30, 120))
    M.set_objective(lin([rng.randint(-3, 20) for _ in range(n)], x), sense="maximize")
    return M


def transport(rng, ns=60, nd=100):
    M = Model()
    x = [[Variable(float, 0, math.inf) for _ in range(nd)] for _ in range(ns)]
    sup = [rng.randint(50, 150) for _ in range(ns)]
    dem = [rng.randint(20, 80) for _ in range(nd)]
    tot = sum(sup)
    while sum(dem) > tot:
        dem[rng.randrange(nd)] -= 1
    for i in range(ns):
        M.add_constraint(lin([1] * nd, x[i]) <= sup[i])
    for j in range(nd):
        M.add_constraint(lin([1] * ns, [x[i][j] for i in range(ns)]) >= dem[j])
    obj = 0
    for i in range(ns):
        obj = obj + lin([rng.randint(1, 50) for _ in range(nd)], x[i])
    M.set_objective(obj)
    return M


def infeasible(rng, n=25):
    M = Model()
    x = [Variable(int, 0, 1) for _ in range(n)]
    w = [rng.randint(2, 9) * 2 for _ in range(n)]
    M.add_constraint(lin(w, x) == (sum(w) // 2) | 1)  # 偶数の和は奇数になれない
    M.set_objective(lin([1] * n, x))
    return M


KINDS = [mknap, setcover, facility, gap, lotsizing, genint, transport, infeasible]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--time-limit", type=float, default=10.0)
    ap.add_argument("--scale", type=float, default=2.5, help="問題の大きさの倍率")
    ap.add_argument("--threads", default="1,4", help="解くスレッドの数 (カンマ区切り、各問題をそれぞれで解く)")
    ap.add_argument("--seeds", type=int, default=2, help="種類ごとの問題の数")
    a = ap.parse_args()
    t0 = time.time()
    for th in [int(t) for t in a.threads.split(",")]:
        for k in KINDS:
            for s in range(a.seeds):
                rng = random.Random(f"{k.__name__}-{s}")
                defaults = k.__defaults__ or ()
                args = [max(2, int(v * a.scale)) for v in defaults]
                m = k(rng, *args)
                t = time.time()
                r = m.solve(time_limit=a.time_limit, threads=th, log=False)
                print(f"{k.__name__} seed {s} threads {th}: {r.status} {getattr(r, 'objective', None)} "
                      f"({time.time() - t:.1f}s)", flush=True)
    print(f"total {time.time() - t0:.1f}s")


if __name__ == "__main__":
    main()
