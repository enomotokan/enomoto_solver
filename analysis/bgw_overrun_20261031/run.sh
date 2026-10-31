#!/bin/bash
# 使い方: run.sh 問題名 時間上限 回数 接頭辞
cd /home/user/enomoto_solver
P=$1; T=$2; N=$3; TAG=$4
for i in $(seq 0 $((N-1))); do
  s=$(date +%s.%N)
  ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_SUBMIP_HIGHS=-s ENOMOTO_MIP_SUBMIP_PC_INHERIT=1 ENOMOTO_T_MIP_SUBMIP_RELIABLE=0 ENOMOTO_MIP_BG_CUTS_WAIT=1 \
    python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $T --threads 4 > /tmp/rep/${TAG}_${P}_${T}_$i.log 2>&1
  e=$(date +%s.%N)
  echo "$P $T $i wall $(echo "$e - $s" | bc) $(tail -1 /tmp/rep/${TAG}_${P}_${T}_$i.log | cut -c1-120)" >> /tmp/rep/${TAG}_summary.txt
done
