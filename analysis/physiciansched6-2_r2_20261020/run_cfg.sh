#!/bin/bash
# run_cfg.sh <問題> <秒> <名前> [環境変数=値 ...]  -> logs/<問題>_<名前>_<n>.log
cd "$(dirname "$0")"
p=$1; T=$2; name=$3; shift 3
n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
env ENOMOTO_MIP_THREADS=4 "$@" python3 eno_run.py ../../.miplib_cache/$p.mps $T > logs/${p}_${name}_${n}.log 2>&1
echo "$p $name $n $(tail -1 logs/${p}_${name}_${n}.log)"
