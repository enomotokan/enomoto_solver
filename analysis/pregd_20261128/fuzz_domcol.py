"""PREGD: 支配列を狙った乱数の MIP で HiGHS と最適値を突き合わせる: python fuzz_domcol.py 問題数 [開始の種]
(ENOMOTO_MIP_PREGD を付けて回す)。元の列を作り、それぞれに「同じ列」と「係数を支配の向きにずらした列」を、型 (0-1・一般整数・連続)・
有限の境界・費用を変えて足す (支配する側・される側の両方ができる)。行は ≤・≥・等式・範囲を混ぜる。
食い違い (状態か最適値) と、当方の解が元の行・境界・整数性を満たさないものを bad と数える。"""
import random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from mip_fuzz_vs_highs import highs, ours  # noqa: E402
inf = float("inf")
def gen(seed):
    g = random.Random(seed); nb = g.randint(2, 8); m = g.randint(2, 8)
    cols = []  # (係数の辞書, 型, lo, up, 費用)
    def typ():
        t = g.randrange(3)
        if t == 0: return True, 0.0, 1.0
        if t == 1: l = float(g.randrange(-2, 2)); return True, l, l + g.randint(1, 4)
        l = float(g.randrange(-2, 2)); return False, l, l + g.random() * 4 + 0.5
    for _ in range(nb):
        r = {i: float(g.choice([-3, -2, -1, 1, 1, 1, 2, 3])) for i in g.sample(range(m), g.randint(1, min(m, 4)))}
        it, l, u = typ(); cols.append((r, it, l, u, float(g.randint(-5, 5))))
        for _ in range(g.randint(0, 3)):
            r2 = dict(r)
            if g.random() < 0.5:  # 係数をずらす (どちらの向きにもなる)
                i = g.choice(list(r2)); r2[i] += g.choice([-1.0, 1.0])
                if r2[i] == 0: del r2[i]
            if g.random() < 0.3:
                i = g.randrange(m); r2[i] = r2.get(i, 0.0) + g.choice([-1.0, 1.0])
                if r2[i] == 0: del r2[i]
            it2, l2, u2 = typ() if g.random() < 0.6 else (it, l, u)
            cols.append((r2, it2, l2, u2, cols[-1][4] + g.choice([0, 0, 1, 2, -1])))
    n = len(cols)
    lo = [c[2] for c in cols]; up = [c[3] for c in cols]; cost = [c[4] for c in cols]; it = [c[1] for c in cols]
    x0 = [round(l + (u - l) * g.random()) if t else l + (u - l) * g.random() for l, u, t in zip(lo, up, it)]
    x0 = [min(max(v, l), u) for v, l, u in zip(x0, lo, up)]
    rows, rlo, rup = [], [], []
    for i in range(m):
        r = [(j, c[0][i]) for j, c in enumerate(cols) if i in c[0]]
        if not r: continue
        act = sum(a * x0[j] for j, a in r); t = g.randrange(5)
        if t <= 1: rlo.append(-inf); rup.append(act + g.choice([0, 0, 1, 2.5]))
        elif t == 2: rlo.append(act - g.choice([0, 0, 1, 2.5])); rup.append(inf)
        elif t == 3: rlo.append(act); rup.append(act)
        else: rlo.append(act - 1); rup.append(act + 1)
        rows.append(r)
    return lo, up, cost, it, rows, rlo, rup
def feasible(lp, x, tol=1e-6):
    lo, up, c, it, rows, rlo, rup = lp
    for j, v in enumerate(x):
        if v < lo[j] - tol or v > up[j] + tol or (it[j] and abs(v - round(v)) > tol): return False
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - tol * (1 + abs(l)) or a > u + tol * (1 + abs(u)): return False
    return True
N = int(sys.argv[1]); s0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); st = {}
for s in range(s0, s0 + N):
    lp = gen(s)
    hs, hv = highs(lp)
    os_, ov, x = ours(lp)
    st[hs] = st.get(hs, 0) + 1
    ok = True
    if hs == "Optimal":
        ok = os_ == "optimal" and abs(ov - hv) <= 1e-6 * (1 + abs(hv)) and feasible(lp, x)
    elif hs == "Infeasible":
        ok = os_ == "infeasible"
    if not ok:
        bad += 1; print("BAD", s, hs, hv, os_, ov, flush=True)
print(f"bad {bad} of {N} statuses {st} time {time.time() - t0:.1f}")
