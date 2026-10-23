# ENOMOTO を 1 回: python run1.py <mps> <時間上限> <スレッド数>
import sys, time, os
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, T, th = sys.argv[1], float(sys.argv[2]), int(sys.argv[3])
_, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp)
t0 = time.time()
nl = int(os.environ['NODE_LIMIT']) if os.environ.get('NODE_LIMIT') else None
r = m.solve(None, False, T, None, nl, th, log=True)
t = time.time() - t0
def g(k):
    try: return getattr(r, k)
    except Exception: pass
    try: return r[k]
    except Exception: return None
print(f"RESULT t={t:.2f} status={g('status')} obj={g('objective')} bound={g('bound') or g('best_bound')} nodes={g('nodes') or g('node_count')}", flush=True)
import resource; ru = resource.getrusage(resource.RUSAGE_SELF)
print(f"CPU user={ru.ru_utime:.1f} sys={ru.ru_stime:.1f}", flush=True)
