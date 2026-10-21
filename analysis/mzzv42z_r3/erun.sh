#!/bin/bash
# ENOMOTO で mzzv42z を 4 スレッドで解く。使い方: erun.sh <名前> [環境変数=値 ...] (T で秒、既定 60)
name=$1; shift
D=$(cd $(dirname "$0") && pwd)
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python3 $D/run_eno.py ${MPS:-/home/user/enomoto_solver/.miplib_cache/mzzv42z.mps} ${T:-60} > $D/logs/e_$name.log 2>&1
echo "$name: $(grep -E '^Root cuts' $D/logs/e_$name.log | head -1 | sed 's/.*-> //') | $(grep RESULT $D/logs/e_$name.log)"
