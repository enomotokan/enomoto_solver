#!/bin/bash
# 同じ LP (HiGHS のラウンド k の後の LP、前処理なし) から ENOMOTO を 1 ラウンド: step2.sh <問題> <HiGHS の名前> <k> [cand|only|none] [環境変数...]
P=$1; H=$2; k=$3; m=$4; shift 4; D=$(cd $(dirname $0) && pwd)
python3 $D/newcuts.py $D/dumps/$H.cuts $k $D/dumps/${H}_new$((k+1)).cuts >/dev/null
E="ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_NO_PRESOLVE=1 ENOMOTO_T_MIP_CUT_ROUNDS=1 ENOMOTO_MIP_CLIQUE_CUTS=off ENOMOTO_MIP_INJECT_CUTS=$D/dumps/$H.cuts ENOMOTO_MIP_INJECT_ROUND=$k"
[ $m != none ] && E="$E ENOMOTO_MIP_INJECT_CAND=$D/dumps/${H}_new$((k+1)).cuts"
[ $m = only ] && E="$E ENOMOTO_MIP_INJECT_ONLY=1"
N=st_${P}_${k}_$m$(echo "$@" | tr -dc 'A-Z0-9' | tail -c 12)
NODE_LIMIT=1 $D/run_eno.sh $P $N 60 1 $E "$@" >/dev/null
h1=$(grep "^HRND $((k+1)) " $D/logs/$H.err|head -1|awk '{print $4}')
echo "$P k=$k $m $*: start $(grep 'INJECT: added' $D/logs/$N.err | awk '{print $10}') ENO1 $(grep 'cut round 0' $D/logs/$N.err | head -1| awk '{print $5, $(NF-1)}') HiGHS1 $h1 | $(grep 'INJECT: round' $D/logs/$N.err | head -1 | cut -d: -f3)"
