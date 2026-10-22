#!/bin/bash
# run_any.sh <問題名> <ログ> <時間上限> <スレッド数> [環境変数...]
P=$1; LOG=$2; T=$3; TH=$4; shift 4
env ENOMOTO_MIP_THREADS=$TH ENOMOTO_MIP_LOG=1 "$@" python3 analysis/tree_replay_20261022/run1.py .miplib_cache/$P.mps $T $TH > $LOG.out 2> $LOG.err
echo "$P $(basename $LOG): $(grep RESULT $LOG.out)"
