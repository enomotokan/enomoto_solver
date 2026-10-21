import sys, time, os
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, T = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp)
t0 = time.time()
r = m.solve(None, False, T, None, None, 4, log=True)
t = time.time() - t0
def g(k):
    for a in (k,):
        try: return getattr(r, a)
        except Exception: pass
    try: return r[k]
    except Exception: return None
print(f"RESULT t={t:.2f} status={g('status')} obj={g('objective')} bound={g('bound') or g('best_bound')} nodes={g('nodes') or g('node_count')}", flush=True)
import resource; ru = resource.getrusage(resource.RUSAGE_SELF)
print(f"CPU user={ru.ru_utime:.1f} sys={ru.ru_stime:.1f} maxrss={ru.ru_maxrss//1024}MB", flush=True)
