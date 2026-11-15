#!/bin/bash
# 根だけを見る短い回: root1.sh 名前 スレッド数 種 [環境変数...]  (時間の上限 1.5 秒)
name=$1; th=$2; seed=$3; shift 3
cd "$(dirname "$0")"; mkdir -p root
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$seed "$@" python ../../../scripts/run_miplib_benchmark.py --worker-ours ${MPS:-../../../.miplib_cache/neos859080.mps} --time-limit 1.5 --threads $th > root/$name.out 2> root/$name.log
rb=$(grep -o '"root_bound": [0-9.e-]*' root/$name.out | cut -d' ' -f2)
g=$(grep -o "cut selection probe (6 rounds): default [0-9]* cuts gain [-0-9.e]*" root/$name.log | awk '{print $NF}')
echo "$name th=$th seed=$seed root_bound=$rb probe_gain=$g"
