#!/bin/bash
# puriri の c0 の根の 0 ラウンド目のカットの LP (1 反復 2.8 ms と遅い) の間、gdb で主スレッドの backtrace を 20 回取る (貧者のプロファイラ)
cd "$(dirname "$0")"
L=/tmp/claude-0/prof_puriri.log
env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_HO4_CUT0=1 python ../../../scripts/run_miplib_benchmark.py --worker-ours ../../../.miplib_cache/neos-3216931-puriri.mps --time-limit 600 --threads 4 > /dev/null 2> $L &
until grep -q "HO4_CUT0" $L; do sleep 1; done
sleep 8
P=$(pgrep -f "worker-ours.*puriri" | head -1)
mkdir -p gdb
for i in $(seq 1 20); do
  gdb -p $P -batch -ex "thread apply all bt 30" 2>/dev/null | grep -E "^#|^Thread" > gdb/puriri_cutlp_$i.txt
  sleep 2
done
grep -E "cut round 0" $L > gdb/puriri_cutlp_log.txt
kill $P; wait
