#!/bin/bash
# 1 スレッド版: ab1.sh <タグ> <問題> <秒> [環境変数 ...]
tag=$1; prob=$2; tl=$3; shift 3
cd /home/user/enomoto_solver
mkdir -p runs_nr/logs
env NR_THREADS=1 ENOMOTO_MIP_RESTORE_DIAG=1 "$@" python3 runs_nr/run1.py .miplib_cache/$prob.mps $tl > runs_nr/logs/${prob}_${tag}.json 2> runs_nr/logs/${prob}_${tag}.err
echo "$prob $tag $* : $(cat runs_nr/logs/${prob}_${tag}.json)"
