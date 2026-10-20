#!/bin/bash
# 使い方: run4.sh <名前> <問題> <時間上限> [ENV=VAL ...]   ENOMOTO を 4 スレッドで回して runs4/<名前>.log に書く
name=$1; prob=$2; T=$3; shift 3
out=/home/user/enomoto_solver/runs4/$name.log
env PYTHONPATH=${PYLIB:-} ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python /home/user/enomoto_solver/scripts_inv/run_eno.py $T /home/user/enomoto_solver/.miplib_cache/$prob.mps > $out 2>&1
echo "$name: $(grep -m1 'Root cuts' $out) | $(grep '^RESULT' $out | cut -c1-300)"
