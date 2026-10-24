#!/bin/bash
# run_highs.sh <問題> <秒> <名前> [キー=値 ...] -> logs/<問題>_h<名前>_<n>.log (HiGHS のログ、4 スレッド、mip_rel_gap 0)
cd "$(dirname "$0")"
p=$1; T=$2; name=$3; shift 3
if [ -f "$p" ]; then mps=$p; p=$(basename $p .mps); else mps=../../.miplib_cache/$p.mps; fi
n=1; while [ -e logs/${p}_h${name}_${n}.log ]; do n=$((n+1)); done
python3 highs_log.py $mps $T 4 "$@" > logs/${p}_h${name}_${n}.log 2>&1
echo "$p h$name $n $(tail -1 logs/${p}_h${name}_${n}.log)"
