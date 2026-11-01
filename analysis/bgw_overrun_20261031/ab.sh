#!/bin/bash
# 使い方: ab.sh 問題名 時間上限 回数
# 直した形 (on) と ENOMOTO_MIP_NO_PRESOLVE_STOP=1 (off、直す前と同じ) を交互に回し、上限 + 1.5 秒を過ぎたら gdb でスタックを取る
cd /home/user/enomoto_solver
P=$1; T=$2; N=$3
for i in $(seq 0 $((N-1))); do
  for v in on off; do
    L=/tmp/rep/ab_${v}_${P}_${T}_$i
    if [ $v = off ]; then X="ENOMOTO_MIP_NO_PRESOLVE_STOP=1"; else X="ENOMOTO_MIP_DUMMY=1"; fi
    s=$(date +%s.%N)
    env $X ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_SUBMIP_HIGHS=-s ENOMOTO_MIP_SUBMIP_PC_INHERIT=1 ENOMOTO_T_MIP_SUBMIP_RELIABLE=0 ENOMOTO_MIP_BG_CUTS_WAIT=1 \
      python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $T --threads 4 > $L.log 2>&1 &
    pid=$!
    while kill -0 $pid 2>/dev/null && [ $(echo "$(date +%s.%N) - $s < $T + 1.5" | bc) = 1 ]; do sleep 0.2; done
    k=0
    while kill -0 $pid 2>/dev/null && [ $k -lt 8 ]; do
      echo "=== stack $k at $(echo "$(date +%s.%N) - $s" | bc)s" >> $L.gdb
      timeout 20 gdb -p $pid -batch -ex "thread apply all bt 25" >> $L.gdb 2>&1
      k=$((k+1)); sleep 1
    done
    wait $pid
    e=$(date +%s.%N)
    echo "$v $P $T $i wall $(echo "$e - $s" | bc) stacks $k maxjoin $(grep -o 'joined [0-9.]*' $L.log | awk '{if($2>m)m=$2}END{print m+0}') $(tail -1 $L.log | cut -c1-110)" >> /tmp/rep/ab_summary.txt
  done
done
