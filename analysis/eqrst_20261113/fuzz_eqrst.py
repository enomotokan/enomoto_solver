"""再スタート後の EQUIV2 の経路を通す乱数の MIP を作り、HiGHS と突き合わせる。
使い方: python fuzz_eqrst.py 問題数 [開始seed] [ログ]  (環境変数は呼び出し側で置く。ENOMOTO_MIP_LOG=1 で置けば、
ログから「再スタートが起きた」「再スタート後に組を使った」を数える。ログは 3 番目の引数のファイルに書く)
問題: 0-1 列 (unit commitment 風に時刻ごとの稼働 x[u,t]・立ち上げ s[u,t])、需要の行 (連続の出力 p[u,t] <= cap x[u,t])、
立ち上げの行 s[u,t] >= x[u,t] - x[u,t-1]、最小稼働の行、ランダムな 2・3 列の論理の行 (x_a <= x_b + z、x_a + x_b <= 1 + z 等、z は費用付きの 0-1)。
判定: 状態と最適値 (相対 1e-6) が HiGHS と一致するか。食い違えば返した点を許容誤差 1e-6 で確かめて印字する。"""
import random, sys, os, time, re
from pathlib import Path
spec = str(Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py")
src = open(spec).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")

def gen(seed):
    g = random.Random(seed)
    U = g.randint(4, 10); T = g.randint(8, 20)
    lo, up, c, it = [], [], [], []
    def var(l, u, cost, integ):
        lo.append(l); up.append(u); c.append(cost); it.append(integ); return len(lo) - 1
    rows, rlo, rup = [], [], []
    def row(r, l, u):
        rows.append(r); rlo.append(l); rup.append(u)
    cap = [g.randint(3, 10) for _ in range(U)]
    fc = [round(g.uniform(1, 20), 2) for _ in range(U)]
    vc = [round(g.uniform(0.5, 5), 2) for _ in range(U)]
    sc = [round(g.uniform(0, 30), 2) for _ in range(U)]
    x = [[var(0, 1, fc[u], True) for t in range(T)] for u in range(U)]
    s = [[var(0, 1, sc[u], True) for t in range(T)] for u in range(U)]
    p = [[var(0, cap[u], vc[u], False) for t in range(T)] for u in range(U)]
    tot = sum(cap)
    for t in range(T):
        d = round(g.uniform(0.2, 0.7) * tot, 1)
        row([(p[u][t], 1.0) for u in range(U)], d, inf)
        for u in range(U):
            row([(p[u][t], 1.0), (x[u][t], -float(cap[u]))], -inf, 0.0)
            if g.random() < 0.5:
                row([(p[u][t], 1.0), (x[u][t], -1.0)], 0.0, inf)  # 最小出力 1
    for u in range(U):
        for t in range(T):
            prev = [(x[u][t - 1], 1.0)] if t > 0 else []
            row([(s[u][t], 1.0), (x[u][t], -1.0)] + prev, 0.0, inf)
        L = g.randint(1, 3)
        for t in range(1, T):  # 最小稼働: s[u,t] = 1 なら x[u,t..t+L-1] = 1
            for k in range(1, L):
                if t + k < T:
                    row([(x[u][t + k], 1.0), (s[u][t], -1.0)], 0.0, inf)
    allx = [v for r in x for v in r] + [v for r in s for v in r]
    for _ in range(g.randint(0, 3 * U)):  # 費用付きの z で緩めた論理の行
        a, b = g.sample(allx, 2); z = var(0, 1, round(g.uniform(0, 15), 2), True)
        k = g.randrange(4)
        if k == 0: row([(a, 1.0), (b, -1.0), (z, -1.0)], -inf, 0.0)
        elif k == 1: row([(a, 1.0), (b, 1.0), (z, -1.0)], -inf, 1.0)
        elif k == 2: row([(a, 1.0), (b, 1.0), (z, 1.0)], 1.0, inf)
        else: row([(a, 1.0), (b, -1.0), (z, 1.0)], 0.0, 0.0) if g.random() < 0.3 else row([(a, 1.0), (b, -1.0), (z, -1.0)], -inf, 0.0)
    if g.random() < 0.5:  # 幅の小さな連続列 (前の誤りの形)
        for _ in range(g.randint(1, 4)):
            y = var(0, 1e-9 * g.choice([1, 2]), -round(g.uniform(0, 300), 2), False)
            a = g.choice(allx); row([(a, 1.0), (y, 1.0)], -inf, 1.0 - 1e-9 * g.randint(0, 2))
    return lo, up, c, it, rows, rlo, rup

def feas(lp, x, tol=1e-6):
    lo, up, c, it, rows, rlo, rup = lp
    if x is None: return "点なし"
    for j, v in enumerate(x):
        if v < lo[j] - tol or v > up[j] + tol: return f"境界 x{j}={v}"
        if it[j] and abs(v - round(v)) > tol: return f"整数性 x{j}={v}"
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - tol or a > u + tol: return f"行 {a} not in [{l},{u}]"
    return "ok"

N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
logf = sys.argv[3] if len(sys.argv) > 3 else None
if logf:  # Rust の eprintln を含む fd 2 をファイルへ
    fd = os.open(logf, os.O_WRONLY | os.O_CREAT | os.O_TRUNC); os.dup2(fd, 2)
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    os.write(2, f"@@SEED {seed}\n".encode())
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        print("seed", seed, "highs", hs, ho, "ours", os_, oo, feas(lp, x) if os_ == "optimal" else "", flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0), flush=True)
