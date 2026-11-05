#!/bin/bash
# 根だけを見る短い回: root1.sh 問題 名前 スレッド数 種 時間の上限 [環境変数...] (ログは root/名前.log)
prob=$1; name=$2; th=$3; seed=$4; tl=$5; shift 5
cd "$(dirname "$0")"
mkdir -p root
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$seed "$@" python ../../scripts/run_miplib_benchmark.py --worker-ours ../../.miplib_cache/$prob.mps --time-limit $tl --threads $th > root/$name.out 2> root/$name.log
echo "$name $(grep -c 'optface' root/$name.log) optface lines; $(tail -1 root/$name.out | head -c 200)"
