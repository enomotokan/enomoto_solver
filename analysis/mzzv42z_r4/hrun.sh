#!/bin/bash
# HiGHS で mzzv42z を解く。使い方: hrun.sh <名前> [opt=val ...]
name=$1; shift
D=$(cd $(dirname "$0") && pwd)
python3 $D/run_highs.py ${MPS:-/home/user/enomoto_solver/.miplib_cache/mzzv42z.mps} ${T:-60} "$@" > $D/logs/h_$name.log 2>&1
echo "$name: $(grep RESULT $D/logs/h_$name.log)"
