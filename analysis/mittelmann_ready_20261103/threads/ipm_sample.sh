#!/bin/bash
# ns1116954 を 1 スレッドで解き、interior rounding の間の解いているスレッドのスタックを 1 秒ごとに取る
S=${OUT:-/tmp}
cd /home/user/enomoto_solver
ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/ns1116954.mps --time-limit ${TL:-60} --threads 1 > $S/ipm.out 2> $S/ipm.err &
PID=$!
until grep -q 'randomized rounding' $S/ipm.err 2>/dev/null || ! kill -0 $PID 2>/dev/null; do sleep 0.5; done
: > $S/ipm.bt
for i in $(seq 1 20); do
  kill -0 $PID 2>/dev/null || break
  echo "=== $(date +%s.%N) sample $i" >> $S/ipm.bt
  gdb -p $PID -batch -ex 'thread apply all bt 30' 2>/dev/null | grep -E '^Thread|#[0-9]+ .* in enomoto|#[0-9]+ +enomoto|interior_point|boxed|kkt|multifrontal|amd|order|crossover' | head -60 >> $S/ipm.bt
  sleep 1
done
wait $PID
