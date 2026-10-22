"""scripts/mip_fuzz_vs_highs.py の問題で、上限で終わるときの判定 (finish の打ち切り値の判定) が誤った最適を出さないかを見る。
ENOMOTO_MIP_NODE_LIMIT などを環境変数で与えて走らせる。最適と報告したら HiGHS の最適値と一致すること、
実行不能と報告したら HiGHS も実行不能であること、上限で終わったら下界 <= HiGHS の最適値 であることを確かめる。
使い方: python fuzz_limits.py N [start]"""
import sys, time
from pathlib import Path
sys.argv_saved = sys.argv; sys.argv = [sys.argv[0], "0"]
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import mip_fuzz_vs_highs as F
sys.argv = sys.argv_saved
from enomoto_solver import Model, Variable
inf = float("inf")
def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    bb = getattr(s, "best_bound", None)
    if bb is None:
        try: bb = s.mip["best_bound"]
        except Exception: bb = None
    return s.status, s.objective, bb
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; cnt = {}; t0 = time.time()
for seed in range(start, start + N):
    lp = F.gen(seed)
    hs, ho = F.highs(lp); os_, oo, bb = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    cnt[os_] = cnt.get(os_, 0) + 1
    if os_ in ("optimal", "infeasible", "unbounded"):
        ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    else:
        ok = hmap != "optimal" or bb is None or bb <= ho + 1e-6 * (1 + abs(ho))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, bb, flush=True)
print("bad", bad, "of", N, "statuses", cnt, "time %.1f" % (time.time() - t0))
