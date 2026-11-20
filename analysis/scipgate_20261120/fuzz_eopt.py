"""試作 ENOMOTO_MIP_VBCLQ=e の「VB01 の解が目的の最小値に届けば最適で打ち切る」道 (README 15 節の (a)) を通しやすい小さな MIP を作り、
HiGHS と最適値・状態を突き合わせる。0 費用の列で被覆の行を満たせ、最適値が目的の最小値 (費用が負の列を 1、正の列を 0) に一致しやすい形
(詰め込みの行・被覆の行・普通のナップサック、費用は正・0・負が混ざる)。
使い方: ENOMOTO_MIP_VBCLQ=e ENOMOTO_MIP_VBCLQ_COUNT_FILE=<ファイル> python fuzz_eopt.py 問題数 [開始seed]
(数えたファイルの印 eopt の行がこの道を通った回)"""
import random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
src = open(WT / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); n = g.randint(6, 30)
    lo = [0.0] * n; up = [1.0] * n; it = [True] * n
    # 費用: 4 割が 0、3 割が正、3 割が負 (負の列は詰め込みの行に入れて、全部 1 にはできないこともある)
    c = [0.0 if g.random() < 0.4 else (round(g.uniform(1, 9), 1) if g.random() < 0.5 else -round(g.uniform(1, 9), 1)) for _ in range(n)]
    rows = []; rlo = []; rup = []
    zero = [j for j in range(n) if c[j] == 0.0] or [0]
    for _ in range(g.randint(1, 6)):  # 被覆の行 (0 費用の列を必ず 1 本含める)
        k = g.sample(range(n), g.randint(2, min(6, n))); k = list(set(k + [g.choice(zero)]))
        rows.append([(j, 1.0) for j in k]); rlo.append(1.0); rup.append(inf)
    for _ in range(g.randint(0, 5)):  # 詰め込みの行
        k = g.sample(range(n), g.randint(2, min(6, n)))
        rows.append([(j, 1.0) for j in k]); rlo.append(-inf); rup.append(1.0)
    for _ in range(g.randint(0, 2)):  # 大 M の行 (y = 1 なら他は 0)
        k = g.sample(range(n), g.randint(3, min(8, n))); y, rest = k[0], k[1:]
        rows.append([(y, float(len(rest)))] + [(j, 1.0) for j in rest]); rlo.append(-inf); rup.append(float(len(rest)))
    for _ in range(g.randint(0, 2)):  # 普通のナップサック
        k = g.sample(range(n), g.randint(2, n)); rows.append([(j, float(g.randint(1, 9))) for j in k]); rlo.append(-inf); rup.append(float(g.randint(5, 25)))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
