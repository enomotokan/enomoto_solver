#!/bin/bash
# 使い方: run1.sh 問題 名前 スレッド数 種 時間の上限 [追加の環境変数...]  (結果は 名前.out・名前.log)
prob=$1; name=$2; th=$3; seed=$4; tl=$5; shift 5
cd "$(dirname "$0")"
mkdir -p runs
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$seed "$@" python ../../scripts/run_miplib_benchmark.py --worker-ours ../../.miplib_cache/$prob.mps --time-limit $tl --threads $th > runs/$name.out 2> runs/$name.log
echo "$name $(tail -1 runs/$name.out | head -c 300)"
