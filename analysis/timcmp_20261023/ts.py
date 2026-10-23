import sys, time
t0 = time.perf_counter()
for line in sys.stdin:
    sys.stdout.write(f"[{time.perf_counter()-t0:7.2f}] {line}"); sys.stdout.flush()
