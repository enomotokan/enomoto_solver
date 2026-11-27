#!/bin/bash
# 1 問を当方 1 スレッド・種 1 で解く: bash run1.sh 問題 秒 名前 [環境変数=値 ...] → logs/<問題>_<名前>.log・.out と、1 行の結果
D=$(cd "$(dirname "$0")" && pwd); WT=$(cd "$D/../.." && pwd); P=$1; TL=$2; N=$3; shift 3
env "$@" ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_T_MIP_SEED=1 timeout $((TL+300)) $WT/.venv/bin/python $WT/scripts/run_miplib_benchmark.py --worker-ours $WT/.miplib_cache/$P.mps --time-limit $TL --threads 1 > $D/logs/${P}_$N.out 2> $D/logs/${P}_$N.log
echo "$P $N $(grep '^{' $D/logs/${P}_$N.out | tail -1 | cut -c1-140)"
