#!/bin/sh
# 1 問を診断つき (ENOMOTO_MIP_POOL3_DIAG・ENOMOTO_MIP_LBNODE_DIAG・ENOMOTO_MIP_LOG) で解く。
# 使い方: sh run_one.sh <問題> <def|p3> <秒> <スレッド数> [名前の後ろ]
# 結果は <問題>_<設定>_<秒>s_t<スレッド>.out (worker の JSON) と .err.gz (ログ)。p3 は ENOMOTO_MIP_POOL3_BEST=5。
D=$(dirname "$0"); W=$D/../..
P=$1; C=$2; T=$3; N=$4; NAME=${P}_${C}_${T}s_t${N}$5
EXTRA=""
[ "$C" = p3 ] && EXTRA="ENOMOTO_MIP_POOL3_BEST=5"
env ENOMOTO_MIP_POOL3_DIAG=1 ENOMOTO_MIP_LBNODE_DIAG=1 ENOMOTO_MIP_LOG=1 $EXTRA \
  python "$W/scripts/run_miplib_benchmark.py" --worker-ours "$W/.miplib_cache/$P.mps" --time-limit "$T" --threads "$N" \
  > "$D/$NAME.out" 2> "/tmp/claude-0/$NAME.err"
gzip -c "/tmp/claude-0/$NAME.err" > "$D/$NAME.err.gz"
