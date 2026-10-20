# 子プロセス (run_eno.py) の出力の各行に開始からの経過秒を付ける。使い方: python ts_run.py <log> <cmd...>
import sys, subprocess, time
log = open(sys.argv[1], "w")
t0 = time.time()
p = subprocess.Popen(sys.argv[2:], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
for line in p.stdout:
    log.write(f"[{time.time()-t0:7.2f}] {line}")
    log.flush()
p.wait()
