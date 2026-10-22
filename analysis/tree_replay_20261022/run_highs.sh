#!/bin/bash
# HiGHS (計装版) を 1 回: run_highs.sh <オプションファイル> <ログ> [追加の環境変数...]
OPT=$1; LOG=$2; shift 2
S=/tmp/claude-0/-home-user-enomoto-solver/c9aa1e48-b774-504a-9a76-b54127b68657/scratchpad
LD_LIBRARY_PATH=$S/highs/build/lib env "$@" HIGHS_PATH_LOG=1 $S/highs/build/bin/highs --model_file .miplib_cache/neos-2978193-inde.mps --options_file $OPT > $LOG.out 2> $LOG.err
grep -E "Nodes|Primal bound|Dual bound|Solving report|Timing|WALL" $LOG.out $LOG.err | head
