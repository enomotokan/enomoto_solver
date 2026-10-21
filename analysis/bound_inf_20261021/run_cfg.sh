#!/bin/bash
# 使い方: run_cfg.sh <問題> <秒> <名前> [環境変数...]  → logs/<問題>_<名前>_<回>.log
cd "$(dirname "$0")"
prob=$1; secs=$2; name=$3; shift 3
k=1; while [ -e logs/${prob}_${name}_$k.log ]; do k=$((k+1)); done
log=logs/${prob}_${name}_$k.log
env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python run_one.py ../../.miplib_cache/$prob.mps $secs 4 > $log 2>&1
grep RESULT $log
