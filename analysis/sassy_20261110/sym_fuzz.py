"""対称な小さな MIP の fuzz (2026-11-10): 同じ形のブロック k 個 (列・行・1 行だけの葉の列) と、ブロックの対応する列を同じ係数で
結ぶ行を作り、列・行の順をでたらめに並べ替えて HiGHS と最適値を突き合わせる。3 割の問題は 1 つのブロックの 1 列の費用だけを
少し変えて対称性を崩す (偽の生成元を作ると誤答になる)。ENOMOTO の設定は環境変数のまま (ENOMOTO_MIP_SYMPRE=probe など)。
使い方: python sym_fuzz.py 問題数 [開始seed]"""
import importlib.util, random, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("fz", str(WT / "scripts/mip_fuzz_vs_highs.py"))
src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec.origin, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")

def gen(seed):
    g = random.Random(seed)
    k = g.randint(3, 7); nb = g.randint(4, 9); mb = g.randint(3, 6); nl = g.randint(0, 4)
    blo, bup, bc, bit = [], [], [], []
    for j in range(nb):
        it = g.random() < 0.7
        u = g.choice([1, 1, 2, 3]) if it else g.choice([1.0, 2.5])
        blo.append(0); bup.append(u); bc.append(g.choice([-3, -2, -1, 1, 2]) + (0 if it else 0.5)); bit.append(it)
    brows = []
    for i in range(mb):
        r = [(j, g.choice([1, 1, 2, -1, 3, 5])) for j in range(nb) if g.random() < 0.6] or [(0, 1)]
        brows.append((r, -inf if g.random() < 0.6 else g.randint(0, 2), g.randint(2, 7)))
    leaf = [(g.randrange(mb), g.choice([1, 2]), g.choice([-1, 1, 2])) for _ in range(nl)]  # (行, 係数, 費用)
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    for b in range(k):
        base = len(lo)
        for j in range(nb):
            lo.append(blo[j]); up.append(bup[j]); c.append(bc[j]); it.append(bit[j])
        lrows = {i: [] for i in range(mb)}
        for (i, a, cc) in leaf:
            lrows[i].append((len(lo), a)); lo.append(0); up.append(1); c.append(cc); it.append(True)
        for i, (r, l, u) in enumerate(brows):
            rows.append([(base + j, a) for j, a in r] + lrows[i]); rlo.append(l); rup.append(u)
    stride = nb + nl
    for _ in range(g.randint(2, 5)):
        sub = [j for j in range(nb) if g.random() < 0.5] or [0]
        a = g.choice([1, 2])
        rows.append([(b * stride + j, a) for b in range(k) for j in sub]); rlo.append(-inf if g.random() < 0.5 else g.randint(1, k)); rup.append(g.randint(k, 2 * k * len(sub)))
    if g.random() < 0.3:
        j = g.randrange(len(c)); c[j] += g.choice([-1, 1]) * 0.25
    # 並べ替え
    pc = list(range(len(lo))); g.shuffle(pc); inv = {old: new for new, old in enumerate(pc)}
    lo2 = [lo[o] for o in pc]; up2 = [up[o] for o in pc]; c2 = [c[o] for o in pc]; it2 = [it[o] for o in pc]
    order = list(range(len(rows))); g.shuffle(order)
    rows2 = [[(inv[j], a) for j, a in rows[i]] for i in order]
    return lo2, up2, c2, it2, rows2, [rlo[i] for i in order], [rup[i] for i in order]

N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); stat = {}
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    stat[hmap] = stat.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo)
print("bad", bad, "of", N, stat, "time %.1f" % (time.time() - t0))
