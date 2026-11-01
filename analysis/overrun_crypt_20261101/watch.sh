#!/bin/bash
# 使い方: watch.sh 問題名 時間上限 回数 接頭辞 [環境変数 ...]
# ENOMOTO_MIP_LOG=1・4 スレッドで回し、起動から上限 + 1.5 秒を過ぎても終わらなければ 1 秒ごとに gdb で全スレッドのスタックを取る (最大 8 回)。
# ログは /tmp/rep/接頭辞_問題_上限_回.log (.gdb)、1 行のまとめは /tmp/rep/接頭辞_summary.txt
cd /home/user/enomoto_solver
P=$1; T=$2; N=$3; TAG=$4; shift 4
for i in $(seq 0 $((N-1))); do
  L=/tmp/rep/${TAG}_${P}_${T}_$i
  s=$(date +%s.%N)
  env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" \
    python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $T --threads 4 > $L.log 2>&1 &
  pid=$!
  while kill -0 $pid 2>/dev/null && [ $(echo "$(date +%s.%N) - $s < $T + 1.5" | bc) = 1 ]; do sleep 0.2; done
  k=0
  while kill -0 $pid 2>/dev/null && [ $k -lt 8 ]; do
    echo "=== stack $k at $(echo "$(date +%s.%N) - $s" | bc)s" >> $L.gdb
    timeout 20 gdb -p $pid -batch -ex "thread apply all bt 30" >> $L.gdb 2>&1
    k=$((k+1)); sleep 1
  done
  wait $pid
  e=$(date +%s.%N)
  echo "$P $T $i wall $(echo "$e - $s" | bc) stacks $k $(tail -1 $L.log | cut -c1-140)" >> /tmp/rep/${TAG}_summary.txt
done
