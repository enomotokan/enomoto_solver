#!/bin/bash
# run_highs.sh <名前> <乱数の種> [時間上限 (既定 600)] [HIGHS_TRACE の水準 (既定 2)]
# 計測用に手を入れた HiGHS 1.15.1 (highs_trace.patch) の highs で neos-960392 を解く。
# 標準出力 (通常のログ) は logs/<名前>.log、計測の行 (TR ...) は logs/<名前>.tr。
cd "$(dirname "$0")"
name=$1; seed=$2; T=${3:-600}; lv=${4:-2}
HIGHS=${HIGHS_BIN:-/tmp/claude-0/-home-user-enomoto-solver/43d652b2-1dda-59a8-ae29-e9eddda32220/scratchpad/highs-src/build/bin/highs}
mps=../../.miplib_cache/neos-960392.mps
opt=logs/$name.opt
printf "time_limit = %s\nrandom_seed = %s\n" $T $seed > $opt
HIGHS_TRACE=$lv $HIGHS --options_file $opt --solution_file logs/$name.sol $mps > logs/$name.log 2> logs/$name.tr
tail -3 logs/$name.log
