#!/bin/bash
# run_eno.sh <名前> <秒> [環境変数=値 ...] -> logs/e_<名前>_<n>.log (ENOMOTO_MIP_LOG=1・4 スレッド)。最後の行に JSON。
cd "$(dirname "$0")"
name=$1; T=$2; shift 2
M=../../../.miplib_cache/physiciansched6-2.mps
n=1; while [ -e logs/e_${name}_${n}.log ]; do n=$((n+1)); done
env ENOMOTO_MIP_LOG=1 "$@" python3 eno_run.py $M $T 4 > logs/e_${name}_${n}.log 2>&1
echo "$name $n $(tail -1 logs/e_${name}_${n}.log)"
