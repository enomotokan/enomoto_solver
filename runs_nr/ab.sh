#!/bin/bash
# 使い方: ab.sh <タグ> <問題> <秒> [環境変数 ...]  -> runs_nr/logs/<問題>_<タグ>.{json,err}
tag=$1; prob=$2; tl=$3; shift 3
cd /home/user/enomoto_solver
mkdir -p runs_nr/logs
env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_RESTORE_DIAG=1 ENOMOTO_PROF_TREE_PAR=1 "$@" python3 runs_nr/run1.py .miplib_cache/$prob.mps $tl > runs_nr/logs/${prob}_${tag}.json 2> runs_nr/logs/${prob}_${tag}.err
echo "$prob $tag $* : $(cat runs_nr/logs/${prob}_${tag}.json)"
