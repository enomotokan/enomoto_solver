"""ENOMOTO を 1 回解いて結果を 1 行で出す。使い方: run_eno.py MPS 時間 木のスレッド数 (環境変数は呼ぶ側で与える)"""
import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, T, th = sys.argv[1], float(sys.argv[2]), int(sys.argv[3])
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t0 = time.time()
r = m.solve(None, False, T, None, None, th, log=True)
dt = time.time() - t0
def g(k):
    try:
        return getattr(r, k)
    except Exception:
        return None
print(f"RESULT t={dt:.1f} status={g('status')} obj={g('objective_value') or g('objective')} bound={g('best_bound') or g('dual_bound')} nodes={g('nodes') or g('node_count')}", flush=True)
