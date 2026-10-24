#!/bin/bash
# run_eno.sh <名前> <秒> [環境変数=値 ...] -> logs/<名前>.log (ENOMOTO_MIP_LOG=1、TH スレッド (既定 4))
cd "$(dirname "$0")"
name=$1; T=$2; shift 2
env ENOMOTO_MIP_THREADS=${TH:-4} ENOMOTO_MIP_LOG=1 "$@" python3 eno_run.py ../../.miplib_cache/neos-960392.mps $T ${TH:-4} > logs/$name.log 2>&1
echo "$name $(tail -1 logs/$name.log)"
