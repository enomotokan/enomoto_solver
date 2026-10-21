# 使い方: python run1.py <mps> <スレッド> <秒>  (1 行の要約を標準出力、求解のログは標準エラー)
import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, th, tl = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.time()
r = m.solve(None, False, tl, None, None, th, log=True)
dt = time.time() - t
def g(k):
    try:
        return r[k]
    except Exception:
        return getattr(r, k, None)
print(f"RESULT {Path(mps).stem} status={g('status')} obj={g('objective')} bound={g('best_bound')} nodes={g('nodes')} time={dt:.2f}", flush=True)
