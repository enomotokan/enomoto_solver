#!/bin/bash
# run.sh eno|highs <問題> <秒> <名前> [ENV=VAL ... | highs のキー=値 ...] -> logs/<問題>_<名前>_<n>.log
# 各行の頭に壁時計の経過秒をつける (ts.py)。ENOMOTO は ../heur_ls_20261023/eno_run.py、HiGHS は highs_log.py
cd "$(dirname "$0")"
kind=$1; p=$2; T=$3; name=$4; shift 4
mps=../../.miplib_cache/$p.mps
n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
out=logs/${p}_${name}_${n}.log
if [ $kind = eno ]; then
  env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python3 -u ../heur_ls_20261023/eno_run.py $mps $T 4 2>&1 | python3 -u ts.py > $out
else
  python3 -u ../heur_ls_20261023/highs_log.py $mps $T 4 "$@" 2>&1 | python3 -u ts.py > $out
fi
echo "$p $name $n $(tail -1 $out)"
