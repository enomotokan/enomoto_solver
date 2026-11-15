#!/bin/bash
# 使い方: run.sh 出力ディレクトリ 時間の上限 [環境変数=値 ...]
out=$1; tl=$2; shift 2; mkdir -p $out
cd /home/user/enomoto_solver
env ENOMOTO_T_MIP_LP_PROBE_TIME_FRAC=$(python3 -c "print(30/$tl)") ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_POSTSOLVE=1 ENOMOTO_MIP_MEM3_DEBUG_INC=$out/inc "$@" \
  timeout $((tl+150)) python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/uccase12.mps --time-limit $tl --threads 4 > $out/run.out 2> $out/run.log
echo "rc $?" >> $out/run.out
