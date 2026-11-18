import sys, time, os
sys.path.insert(0, 'scripts')
import run_miplib_benchmark as rb
try:
    rb.worker_ours(sys.argv[1], float(sys.argv[2]), 4)
finally:
    sys.stdout.flush()
    print("SOLVE_DONE", time.time(), file=sys.stderr, flush=True)
    time.sleep(12)
