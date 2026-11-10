"""cut-based の衝突解析 (ENOMOTO_MIP_CUTCONF) の誤答の検査用: 木の探索が要る小さな 0-1 混合の問題 (多次元ナップサック、
固定費用の連続列、少しの一般整数列、≥ の被覆の行、等式) を作り、HiGHS と最適値を突き合わせる。
使い方: ENOMOTO_MIP_CUTCONF=1 python fuzz_cutconf.py 問題数 [開始seed]
mip_fuzz_vs_highs.py の highs()/ours() を使う。最適値が相対 1e-6 で一致しなければ bad。"""
import random, sys, time
from pathlib import Path
spec_path = Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"
src = open(spec_path).read().split("N=int(sys.argv[1])")[0]
ns = {}
exec(compile(src, str(spec_path), "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")


def gen(seed, big=False):
    g = random.Random(seed)
    nb = g.randint(40, 90) if big else g.randint(12, 34)
    ni = g.randint(2, 10) if big else g.randint(0, 4)
    nc = g.randint(2, 12) if big else g.randint(0, 6)
    n = nb + ni + nc
    # 一般整数・連続の列は下限が 0 でないこともある (大域的な境界で補にする所を試す)
    lo = [0.0] * nb + [float(g.randint(-3, 2)) for _ in range(ni)] + [-float(g.choice([0, 0, g.randint(1, 5)])) for _ in range(nc)]
    up = [1.0] * nb + [lo[nb + k] + float(g.randint(1, 6)) for k in range(ni)] + [float(g.randint(1, 20)) for _ in range(nc)]
    it = [True] * (nb + ni) + [False] * nc
    c = [-float(g.randint(1, 40)) for _ in range(nb + ni)] + [-round(g.random() * 5, 2) for _ in range(nc)]
    rows, rlo, rup = [], [], []
    for _ in range(g.randint(6, 14) if big else g.randint(2, 7)):
        r = {}
        for j in g.sample(range(nb + ni), g.randint(4, nb + ni)):
            r[j] = float(g.randint(1, 30))
        for j in range(nb + ni, n):
            if g.random() < 0.4:
                r[j] = float(g.randint(1, 10))
        cap = sum(r[j] * lo[j] for j in r) + sum(r[j] * (up[j] - lo[j]) for j in r) * g.uniform(0.25, 0.6)
        rows.append(list(r.items())); rlo.append(-inf); rup.append(float(int(cap)))
    # 固定費用: x_c <= u_c y_b
    for j in range(nb + ni, n):
        if g.random() < 0.7:
            b = g.randrange(nb)
            rows.append([(j, 1.0), (b, -(up[j] - lo[j]))]); rlo.append(-inf); rup.append(lo[j])
    # 被覆の行 (どれか k 個は選ぶ) と、少しの等式
    for _ in range(g.randint(2, 8) if big else g.randint(0, 2)):
        xs = g.sample(range(nb), g.randint(3, min(8, nb)))
        k = g.randint(1, 2)
        rows.append([(j, 1.0) for j in xs]); rlo.append(float(k)); rup.append(inf)
    for _ in range(g.randint(2, 6) if big else g.randint(0, 1)):
        xs = g.sample(range(nb), 4)
        rows.append([(xs[0], 1.0), (xs[1], 1.0), (xs[2], -1.0), (xs[3], -1.0)]); rlo.append(0.0); rup.append(0.0)
    # 衝突 (クリーク) の行
    for _ in range(g.randint(4, 20) if big else g.randint(0, 3)):
        xs = g.sample(range(nb), g.randint(2, 4))
        rows.append([(j, 1.0) for j in xs]); rlo.append(-inf); rup.append(1.0)
    return lo, up, c, it, rows, rlo, rup


if __name__ != "__main__":
    raise ImportError  # fuzz_check.py は gen だけ使う (下の本体は走らせない)
N = int(sys.argv[1])
start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0
t0 = time.time()
for s in range(start, start + N):
    lp = gen(s)
    hs, hv = highs(lp)
    st, ov, _ = ours(lp)
    hst = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    ok = (st == hst) and (st != "optimal" or abs(ov - hv) <= 1e-6 * (1 + abs(hv)))
    if not ok:
        bad += 1
        print("BAD seed", s, "highs", hs, hv, "ours", st, ov, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
