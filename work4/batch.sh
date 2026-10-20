#!/bin/bash
# 使い方: batch.sh <名前> <回数> <時間上限> [VAR=値 ...]  (4 スレッド、ログは work4/b_<名前>_<i>.log)
name=$1; reps=$2; T=$3; shift 3
M=/home/user/enomoto_solver/.miplib_cache/neos-2978193-inde.mps.gz
for i in $(seq 1 $reps); do
  env "$@" ENOMOTO_MIP_THREADS=4 NT=4 python /home/user/enomoto_solver/work4/ts_run.py /home/user/enomoto_solver/work4/b_${name}_$i.log python /home/user/enomoto_solver/work4/run_eno.py $M $T
done
