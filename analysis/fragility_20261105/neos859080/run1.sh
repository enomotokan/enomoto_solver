#!/bin/bash
# 使い方: run1.sh 名前 スレッド数 種 [追加の環境変数...]
name=$1; th=$2; seed=$3; shift 3
cd "$(dirname "$0")"
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$seed "$@" python ../../../scripts/run_miplib_benchmark.py --worker-ours ${MPS:-../../../.miplib_cache/neos859080.mps} --time-limit 600 --threads $th > $name.out 2> $name.log
echo "$name $(tail -1 $name.out)"
