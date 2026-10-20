"""子の CPU 時間を測る: python cputime.py <ラベル> <コマンド ...>。最後に「ラベル user sys wall」を出す。"""
import resource, subprocess, sys, time
t = time.perf_counter()
subprocess.run(sys.argv[2:])
r = resource.getrusage(resource.RUSAGE_CHILDREN)
print(f"{sys.argv[1]} user {r.ru_utime:.1f} sys {r.ru_stime:.1f} wall {time.perf_counter() - t:.1f}", flush=True)
