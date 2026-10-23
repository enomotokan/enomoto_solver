#!/bin/bash
# ENOMOTO を 1 回: run_eno.sh <問題> <名前> <時間上限> <スレッド数> [環境変数...]
# NODE_LIMIT=1 を渡すと根だけ。ログ logs/<名前>.err・.out
P=$1; N=$2; T=$3; TH=$4; shift 4
D=$(cd $(dirname $0) && pwd); R=$D/../..
env ENOMOTO_MIP_THREADS=$TH ENOMOTO_MIP_LOG=1 "$@" python3 $D/run1.py $R/.miplib_cache/$P.mps $T $TH > $D/logs/$N.out 2> $D/logs/$N.err
echo "$N $*: $(grep RESULT $D/logs/$N.out)"
