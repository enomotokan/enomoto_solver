#!/bin/bash
# run_eno.sh <ログ> <時間上限> <スレッド数> [環境変数...]  (打ち切り値つきは ENOMOTO_MIP_CUTOFF=-2.38805 を渡す)
LOG=$1; T=$2; TH=$3; shift 3
env ENOMOTO_MIP_THREADS=$TH ENOMOTO_MIP_LOG=1 "$@" python3 analysis/tree_replay_20261022/run1.py .miplib_cache/neos-2978193-inde.mps $T $TH > $LOG.out 2> $LOG.err
echo "$LOG $*: $(grep RESULT $LOG.out)"
