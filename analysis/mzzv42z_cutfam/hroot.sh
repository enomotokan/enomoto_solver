#!/bin/bash
# 計測用の HiGHS (scripts/highs_instrument.patch + analysis/mzzv42z_cutfam/highs_origin.patch) で mzzv42z の根だけを解く。
# 使い方: hroot.sh <名前> <presolve on|off> [環境変数=値 ...]。根で止めるため mip_max_nodes=1、ヒューリスティクス 0、4 スレッド
name=$1; pre=$2; shift 2
HB=${HIGHS_BIN:-/home/user/hb/HiGHS/build/bin/highs}
D=$(dirname "$0")
opt=$D/logs/opt_$name.txt
printf "presolve=%s\nthreads=4\nmip_max_nodes=1\nmip_heuristic_effort=0\ntime_limit=120\n" "$pre" > $opt
env "$@" $HB --options_file $opt --model_file ${MPS:-/home/user/enomoto_solver/.miplib_cache/mzzv42z.mps} > $D/logs/h_$name.log 2>&1
grep -E "HSEP|HADD|HORIG|HPROP" $D/logs/h_$name.log > $D/logs/h_$name.sep
echo "$name: $(grep -E 'Dual bound' $D/logs/h_$name.log | tail -1)"
