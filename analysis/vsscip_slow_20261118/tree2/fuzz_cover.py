"""群 2 drev の確かめ: 支配される列が多く出る被覆型の小さな 0-1 問題を作り、HiGHS と最適値を突き合わせる。
python fuzz_cover.py 問題数 [開始seed]。環境変数 (ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DOMCOL_PRED=1 ENOMOTO_MIP_DOMCOL_REV=1) は外から付ける。
列は「親」の列の行の部分集合を持つ子を作り (支配の組)、費用は子の方を高くしたり同じにしたりする。≥ の被覆の行のほか、
≤ の行・等式・負の係数・負の費用も少し混ぜる (固定してはいけない組も作る)。"""
import random, sys, time
sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[3] / "scripts"))
import importlib.util
spec = importlib.util.spec_from_file_location("mf", str(__import__("pathlib").Path(__file__).resolve().parents[3] / "scripts/mip_fuzz_vs_highs.py"))
inf = float("inf")

def gen(seed):
    g = random.Random(seed); m = g.randint(3, 9); n0 = g.randint(3, 8)
    cols = []  # (行の集合 {i: a}, 費用)
    for _ in range(n0):
        rs = {i: float(g.choice([1, 1, 1, 2, 3])) for i in range(m) if g.random() < 0.45}
        cols.append((rs, float(g.randint(0, 6))))
    for _ in range(g.randint(2, 10)):  # 子 (支配される列)
        p, c = cols[g.randrange(len(cols))]
        rs = {i: a if g.random() < 0.7 else max(1.0, a - 1) for i, a in p.items() if g.random() < 0.7}
        cc = c + g.choice([0, 0, 1, 2, -1]) if g.random() < 0.9 else -float(g.randint(0, 3))
        cols.append((rs, float(cc)))
    n = len(cols)
    rows = [[(j, cols[j][0][i]) for j in range(n) if i in cols[j][0]] for i in range(m)]
    rlo, rup = [], []
    for i in range(m):
        t = g.random()
        tot = sum(a for _, a in rows[i])
        if t < 0.75: rlo.append(float(g.randint(1, max(1, int(tot // 2) or 1)))); rup.append(inf)
        elif t < 0.9: rlo.append(-inf); rup.append(float(g.randint(1, max(1, int(tot)))))
        else: v = float(g.randint(1, max(1, int(tot // 2) or 1))); rlo.append(v); rup.append(v)
    # 負の係数の行を少し
    for _ in range(g.randint(0, 2)):
        r = {}
        for _ in range(g.randint(2, 4)):
            r[g.randrange(n)] = float(g.choice([-2, -1, 1, 2]))
        rows.append(list(r.items())); rlo.append(-inf if g.random() < 0.5 else -1.0); rup.append(float(g.randint(0, 2)) if g.random() < 0.7 else inf)
    rows = [r for r in rows if r]
    rlo, rup = rlo[:len(rows)], rup[:len(rows)]
    lo = [0.0] * n; up = [1.0] * n; c = [cols[j][1] for j in range(n)]; it = [True] * n
    return lo, up, c, it, rows, rlo, rup

def main():
    import types
    src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
    mf = types.ModuleType("mf"); exec(src, mf.__dict__)
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; t0 = time.time(); st = {}
    for seed in range(start, start + N):
        lp = gen(seed)
        if len(lp[4]) != len(lp[5]):
            continue
        hs, ho = mf.highs(lp); os_, oo, x = mf.ours(lp)
        hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
        st[hmap] = st.get(hmap, 0) + 1
        ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo)
    print("bad", bad, "of", N, "status", st, "time %.1f" % (time.time() - t0))

if __name__ == "__main__":
    main()
