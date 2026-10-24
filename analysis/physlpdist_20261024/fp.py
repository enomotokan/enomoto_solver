"""書き出した根の LP 解から Feasibility Pump を回す: python fp.py <接頭辞k> <形> [目的値の上限 or ''] [反復] [秒] [種]
形: eno = ENOMOTO の `feasibility_pump` と同じ手順 (目的を混ぜる alpha は 1 から 0.9 倍ずつ、丸めの閾値は 0.4〜0.6 の乱数、
          循環したら 10 列をランダムに反転、2 回続けて循環したら終わり)
     std = 標準の改良版 (Fischetti-Glover-Lodi・Bertacchini ら): 循環したら距離の大きい順に T (10〜30) 列を反転、
          反転でも抜けなければ全体を乱数で摂動してやり直す。alpha は eno と同じ
     dist = std から目的を混ぜないもの (alpha = 0)
目的値の上限を与えると、行 c x + offset <= 上限 を LP に足す (最適値を与えれば、最適解だけを探す Pump になる)。
LP は書き出した問題の行だけ (カットは含まない。出発の LP 解はカットも満たす)。整数列はどれも 0-1。
各反復の距離 (LP 解と丸めた点の L1) と、見つかった解の目的値・最適解 (`_opt.txt`・`_near_x.txt`) との距離を出す。"""
import sys, time, os
import numpy as np
import highspy
import scipy.sparse as sp

pre, form = sys.argv[1], sys.argv[2]
zmax = float(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3] else None
ITERS = int(sys.argv[4]) if len(sys.argv) > 4 else 300
T = float(sys.argv[5]) if len(sys.argv) > 5 else 300.0
seed = int(sys.argv[6]) if len(sys.argv) > 6 else 1
rng = np.random.default_rng(seed)

h = highspy.Highs()
h.setOptionValue("output_flag", False)
h.readModel(pre + ".mps")
lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
c = np.array(lp.col_cost_)
off = lp.offset_
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
isint = np.array([int(v) for v in lp.integrality_]) == 1
free = isint & (lo < up)
assert np.all(lo[free] == 0) and np.all(up[free] == 1), "0-1 でない整数列がある"
F = np.nonzero(free)[0]
A = lp.a_matrix_
M = sp.csc_matrix((A.value_, A.index_, A.start_), shape=(m, n)).tocsr()
rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
# LP として解く (整数性を外す)
h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
if zmax is not None:
    idx = np.nonzero(c)[0].astype(np.int32)
    h.addRow(-highspy.kHighsInf, zmax - off + 1e-4, len(idx), idx, c[idx])
h.setOptionValue("presolve", "off")
h.setOptionValue("simplex_strategy", 4)  # 主単体法 (費用だけを変えて続けて解く)
h.setOptionValue("time_limit", T)

x = np.loadtxt(pre + "_x.txt")
refs = {k: np.loadtxt(pre + f"_{f}.txt") for k, f in [("opt", "opt"), ("near", "near_x")] if os.path.exists(pre + f"_{f}.txt")}
cnorm = np.linalg.norm(c[F]) if np.any(c[F]) else np.linalg.norm(c)


def feasible(r, xc):
    """整数列を r、ほかを xc の値にした点の行の最大の逸脱。"""
    y = xc.copy()
    y[F] = r[F]
    ax = M @ y
    return max(0.0, float(np.max(rl - ax)), float(np.max(ax - ru)))


def report(tag, y, it, t0):
    z = c @ y + off
    ds = " ".join(f"L1 to {k} {np.abs(y[F] - v[F]).sum():.0f}" for k, v in refs.items())
    print(f"{tag}: iteration {it}, objective {z:.1f}, L1 to the start LP {np.abs(y[F] - x[F]).sum():.1f}, {ds} ({time.perf_counter() - t0:.1f}s)", flush=True)


t0 = time.perf_counter()
alpha = 0.0 if form == "dist" else 1.0
seen = set()
xs = x.copy()
dists = []
found = None
restarts = 0
for it in range(ITERS):
    if time.perf_counter() - t0 > T:
        break
    xf = xs[F]
    if np.all(np.abs(xf - np.round(xf)) <= 1e-6):
        found = xs.copy()
        report("found (LP solution integral)", found, it, t0)
        break
    if form == "eno":
        th = 0.4 + 0.2 * rng.random(len(F))
        r = np.floor(xf + 1.0 - th)
    else:
        r = np.round(xf)
    d = np.abs(xf - r).sum()
    dists.append(d)
    key = r.astype(np.int8).tobytes()
    if key in seen:
        if form == "eno":
            ok = False
            for _ in range(2):
                for k in rng.integers(0, len(F), 10):
                    r[k] = 1.0 - r[k] if r[k] == round(xf[k]) or xf[k] in (0.0, 1.0) else round(xf[k])
                key = r.astype(np.int8).tobytes()
                if key not in seen:
                    ok = True
                    break
            if not ok:
                print(f"stopped: cycle at iteration {it}")
                break
        else:
            # 距離の大きい順に T 列を反転
            tt = int(rng.integers(10, 31))
            order = np.argsort(-np.abs(xf - r))
            r2 = r.copy()
            r2[order[:tt]] = 1.0 - r2[order[:tt]]
            key = r2.astype(np.int8).tobytes()
            if key in seen:
                # やり直し: |x - r| + 乱数 (-0.3〜0.7、正の部分だけ) が 0.5 を超える列を反転
                rho = rng.uniform(-0.3, 0.7, len(F))
                flip = np.abs(xf - r) + np.maximum(rho, 0) > 0.5
                r2 = r.copy()
                r2[flip] = 1.0 - r2[flip]
                key = r2.astype(np.int8).tobytes()
                restarts += 1
            r = r2
    seen.add(key)
    rr = xs.copy()
    rr[F] = r
    if feasible(rr, xs) <= 1e-6:
        found = rr
        report("found (rounded point feasible)", found, it, t0)
        break
    # 距離の目的 (r = 0 の列は +1、r = 1 の列は -1)
    cost = np.zeros(n)
    cost[F] = np.where(r == 0, 1.0, -1.0)
    if alpha > 1e-3:
        dn = np.linalg.norm(cost)
        cost = (1 - alpha) * cost + alpha * max(dn, 1.0) / cnorm * c
    alpha *= 0.9
    h.changeColsCost(n, np.arange(n, dtype=np.int32), cost)
    h.run()
    st = h.getModelStatus()
    if st != highspy.HighsModelStatus.kOptimal:
        print(f"stopped: LP {h.modelStatusToString(st)} at iteration {it}")
        break
    xs = np.array(h.getSolution().col_value)
    if it % 20 == 0:
        print(f"  it {it}: distance {d:.1f}, fractional {np.sum(np.abs(xs[F] - np.round(xs[F])) > 1e-6)}, objective of the LP point {c @ xs + off:.1f}, alpha {alpha:.3f} ({time.perf_counter() - t0:.1f}s)", flush=True)
print(f"{pre} {form} bound {zmax} seed {seed}: {'found' if found is not None else 'not found'}, iterations {len(dists)}, restarts {restarts}, "
      f"distance first {dists[0] if dists else float('nan'):.1f} min {min(dists) if dists else float('nan'):.1f} last {dists[-1] if dists else float('nan'):.1f} ({time.perf_counter() - t0:.1f}s)")
