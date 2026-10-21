#!/bin/bash
# 使い方: erun.sh <問題> <設定> <回> [追加の環境変数...]。ログは logs/<問題>_<設定>_<回>.log
D=$(cd $(dirname "$0") && pwd); source $D/cfg.sh
p=$1; c=$2; k=$3; shift 3
LOGDIR=${LOGDIR:-$D/logs}
f=$LOGDIR/${p}_${c}_${k}.log
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_DEBUG_FACETIME=1 $(cfg $c) "$@" timeout 200 python3 $D/run_eno.py /home/user/enomoto_solver/.miplib_cache/$p.mps ${T:-60} > $f 2>&1
echo "$(date +%H:%M:%S) $p $c $k $(grep RESULT $f)"
