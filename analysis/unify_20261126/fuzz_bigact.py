"""活動度の大きい行 (|act| が 1e3〜1e6) を持つ小さな MIP を作り、HiGHS と最適値を突き合わせ、返した点の実行可能性も確かめる
(analysis/unify_20261126 の 2 節の 1: 旧 FJ・local_search の違反の許容誤差が活動度に比例して緩く、|act| の大きい行で
`is_feasible` が拒む点を「実行可能」とみなしうる件の誤答の芽を見る)。
使い方: python fuzz_bigact.py 問題数 [開始seed]  (ENOMOTO_MIP_REL_GAP=1e-9 で回す)
形は scripts/mip_fuzz_vs_highs.py の gen を基に、行の係数に 10^k (k = 2〜4) を掛け、整数列の値を大きく取れるようにしたもの。
判定: 状態が同じで最適値が相対 1e-6 で一致し、返した点が許容誤差 1e-6 + 1e-9 |b| (`MipProblem::is_feasible` と同じ) で実行可能なら正しい。"""
import random, sys, time
from pathlib import Path
spec_path = Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"
src = open(spec_path).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, str(spec_path), "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); n = g.randint(3, 16); m = g.randint(2, 10)
    lo, up, c, it = [], [], [], []
    for j in range(n):
        t = g.randrange(4)
        if t == 0: l, u = 0.0, float(g.randint(50, 2000))
        elif t == 1: l, u = 0.0, 1.0
        elif t == 2: l, u = -float(g.randint(0, 100)), float(g.randint(100, 5000))
        else: l, u = 0.0, inf
        lo.append(l); up.append(u); c.append(round((g.random() - 0.5) * 20, 2)); it.append(g.random() < 0.7)
    x0 = []
    for j in range(n):
        v = (lo[j] + (up[j] - lo[j]) * g.random()) if up[j] < inf else lo[j] + 1000 * g.random()
        if it[j]: v = min(max(round(v), lo[j]), up[j])
        x0.append(v)
    rows, rlo, rup = [], [], []
    for i in range(m):
        sc = 10.0 ** g.randint(2, 4)
        r = {}
        for _ in range(2 + g.randrange(min(n, 5))):
            j = g.randrange(n); v = float(round((g.random() - 0.5) * 20)) * sc / 10
            if v != 0: r[j] = v
        r = list(r.items())
        if not r: continue
        act = sum(v * x0[j] for j, v in r)
        t = g.randrange(4)
        w = g.random() * 3 * sc / 100
        if t == 0: rlo.append(-inf); rup.append(act + w)
        elif t == 1: rlo.append(act - w); rup.append(inf)
        elif t == 2: rlo.append(act - w); rup.append(act + w)
        else: rlo.append(act); rup.append(act)
        rows.append(r)
    s = sum(abs(v) for v in x0) + 20
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-inf); rup.append(s)
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-s); rup.append(inf)
    return lo, up, c, it, rows, rlo, rup
def feasible(lp, x, tol=1e-6):
    lo, up, c, it, rows, rlo, rup = lp
    for j, v in enumerate(x):
        if v < lo[j] - tol - 1e-9 * abs(lo[j]) or v > up[j] + tol + 1e-9 * abs(up[j]): return f"境界を破る x{j}={v}"
        if it[j] and abs(v - round(v)) > tol: return f"整数でない x{j}={v}"
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - tol - 1e-9 * abs(l) or a > u + tol + 1e-9 * abs(u): return f"行を破る {a} not in [{l}, {u}]"
    return None
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    why = feasible(lp, x) if (ok and x is not None) else None
    # 最適値が HiGHS より良く、返した点が実行可能なら HiGHS の取りこぼし (seed 88: 係数の大きい行で HiGHS が -15310.65、こちらは
    # 厳密に実行可能な点で -15350.21。HiGHS はギャップ 1e-12 でも同じ値)。印字だけして正しいとみなす
    if not ok and hmap == os_ == "optimal" and x is not None and oo < ho and feasible(lp, x) is None:
        print("highs-worse", seed, "highs", ho, "ours", oo, flush=True)
        ok = True
    if not ok or why:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, why or "", flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
