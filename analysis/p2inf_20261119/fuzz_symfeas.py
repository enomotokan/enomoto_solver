"""P2-INF: SST_FEAS (目的が全部 0 の問題だけ SST) の誤答の確かめ。scripts/mip_fuzz_symmetric.py の問題の目的を全部 0 にし、
仕事の行をちょうど 1 台 (= 1) にして、容量で実行不能になりうる対称な実行可能性問題にする。HiGHS と状態 (optimal / infeasible) を突き合わせ、
optimal なら返した解が行・境界・整数性を満たすかも確かめる。
使い方: ENOMOTO_MIP_SYM_SST_FEAS=1 [ENOMOTO_MIP_NO_ORBITOPE=1] python fuzz_symfeas.py 問題数 [開始seed] [大きさの倍率]"""
import sys, time
from pathlib import Path
src = open(Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_symmetric.py").read().split("N = int(sys.argv[1])")[0]
exec(src)
inf = float('inf')
def gen0(seed):
    lo, up, c, it, rows, rlo, rup = gen(seed)
    c = [0.0] * len(c)
    rlo = list(rlo)
    # 先頭の仕事の行 (係数が全部 1、右辺 1) をちょうど 1 台にする
    for k, r in enumerate(rows):
        if rup[k] == 1.0 and all(v == 1.0 for _, v in r) and len(r) > 2 - 1 and k < len(rows):
            rlo[k] = 1.0
        else:
            break
    # 環境変数 FUZZ_CAP_MULT (既定 1): 容量の行 (右辺が 1 より大きい ≤ の行) の右辺を倍にし、前処理で実行不能と決まる問題を減らす
    mult = float(__import__("os").environ.get("FUZZ_CAP_MULT", "1"))
    rup = [u * mult if u < inf and u > 1.0 else u for u in rup]
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); cnt = {}
for seed in range(start, start + N):
    lp = gen0(seed)
    hs, _ = highs(lp); os_, _, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible'}.get(hs, hs)
    ok = hmap == os_
    if ok and os_ == 'optimal':
        lo, up, c, it, rows, rlo, rup = lp
        tol = 1e-6
        ok = all(lo[j] - tol <= x[j] <= up[j] + tol and (not it[j] or abs(x[j] - round(x[j])) <= tol) for j in range(len(lo)))
        ok = ok and all(rlo[k] - tol <= sum(v * x[j] for j, v in r) <= rup[k] + tol for k, r in enumerate(rows))
    cnt[hmap] = cnt.get(hmap, 0) + 1
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, "ours", os_)
print("bad", bad, "of", N, "HiGHS", cnt, "time %.1f" % (time.time() - t0))
