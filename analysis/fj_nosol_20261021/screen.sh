#!/bin/bash
# screen.sh <名前> [環境変数=値 ...]: 55 問で FJ 用の問題 (FJ_P3) の形と FJ の試行 5 回の結果を集める -> logs/screen_<名前>.txt
cd "$(dirname "$0")"
name=$1; shift
out=logs/screen_${name}.txt; : > $out
for p in $(cat problems56.txt); do
  env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_FJ_TRIALS=5 ENOMOTO_T_MIP_DEBUG_FJ_TRIALS_CAP=5 "$@" \
    timeout 120 python3 eno_run.py ../../.miplib_cache/$p.mps 2 2>&1 | grep "^Presolve:\|FJ problem (FJ_P3)\|FJ trials" | sed "s/^/$p: /" >> $out
done
