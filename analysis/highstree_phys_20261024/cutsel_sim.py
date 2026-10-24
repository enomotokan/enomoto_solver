"""ENOMOTO の選択 (select_cuts_hybrid: 質 = 書き出した効き目で近似、最良の min_rel 倍未満を捨て、平行度 maxpar (良いもの 0.7) で間引く)
を設定を変えて再現し、LP 緩和に足した値を比べる: python -I cutsel_sim.py <mps> <cands_r0.txt> <h_x_r1.txt>"""
import sys, runpy
import numpy as np
mps, ec, hx = sys.argv[1:4]
g = runpy.run_path("cutcmp.py", init_globals={"LIBARGS": [mps, None, hx, ec, None]}, run_name="lib")
lp_with = g["lp_with"]
E = []
for line in open(ec):
    t = line.split(); E.append((int(t[0]) % 16, float(t[1]), float(t[2]), dict((int(a), float(b)) for a, b in (x.split(":") for x in t[3:]))))
def sel(min_rel, maxpar, max_cuts, dens=0.0, srcs=None):
    sc = [(e / (len(c) ** dens), i) for i, (s, e, r, c) in enumerate(E) if srcs is None or s in srcs]
    sc.sort(reverse=True)
    best = sc[0][0]; chosen = []
    for s, i in sc:
        if len(chosen) >= max_cuts or s < min_rel * best:
            break
        c = E[i][3]; nc = np.sqrt(sum(v * v for v in c.values()))
        lim = 0.7 if s >= 0.9 * best else maxpar
        if any(abs(sum(v * d.get(j, 0.0) for j, v in c.items())) > lim * nc * nd for nd, d in chosen):
            continue
        chosen.append((nc, c))
    cuts = [("x", E[i][2], list(E[i][3].items())) for i in [None]] if False else None
    out = []
    for nc, c in chosen:
        k = next(i for i in range(len(E)) if E[i][3] is c)
        out.append(("x", E[k][2], list(c.items())))
    return out
for args in [(0.5, 0.3, 150), (0.5, 0.3, 500), (0.1, 0.3, 150), (0.0, 0.3, 150), (0.0, 0.3, 500), (0.0, 0.9, 150), (0.0, 0.9, 500), (0.5, 0.3, 150, 1.0), (0.0, 0.3, 150, 0.5)]:
    cs = sel(*args)
    print(f"min_rel {args[0]} maxpar {args[1]} 本数上限 {args[2]} 密度の指数 {args[3] if len(args) > 3 else 0}: 採用 {len(cs)} 非零の平均 {np.mean([len(c[2]) for c in cs]):.1f} -> LP {lp_with(cs)[0]:.6g}")
