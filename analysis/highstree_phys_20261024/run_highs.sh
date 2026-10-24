#!/bin/bash
# run_highs.sh <名前> <秒> <乱数の種>  -> logs/<名前>.log (通常のログ)・logs/<名前>.tr (計測の行 "TR ...")・logs/<名前>.sol
# HIGHS_BIN で計測版 HiGHS (highs_trace.patch を当てた v1.15.1) の highs を指す
cd "$(dirname "$0")"
name=$1; T=$2; seed=${3:-0}
mps=../../.miplib_cache/physiciansched6-2.mps
opt=logs/$name.opt
printf "time_limit = %s\nrandom_seed = %s\n" "$T" "$seed" > $opt
HIGHS_TRACE=1 $HIGHS_BIN --options_file $opt --solution_file logs/$name.sol $mps > logs/$name.log 2> logs/$name.tr
grep -E "Status|Primal bound|Dual bound|Nodes|Timing" logs/$name.log | head -8
