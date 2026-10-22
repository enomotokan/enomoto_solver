#!/bin/bash
# 同じ LP (HiGHS のラウンド k の後の LP) から 1 ラウンド: step.sh <k> [cand|only|none] [追加の環境変数...]
k=$1; m=$2; shift 2; D=$(cd $(dirname $0) && pwd)
python3 $D/newcuts.py $D/dumps/roll3000_h_nopre.cuts $k $D/dumps/r3_new$((k+1)).cuts >/dev/null
E="ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_NO_PRESOLVE=1 ENOMOTO_T_MIP_CUT_ROUNDS=1 ENOMOTO_MIP_CLIQUE_CUTS=off ENOMOTO_MIP_INJECT_CUTS=$D/dumps/roll3000_h_nopre.cuts ENOMOTO_MIP_INJECT_ROUND=$k"
[ $m != none ] && E="$E ENOMOTO_MIP_INJECT_CAND=$D/dumps/r3_new$((k+1)).cuts"
[ $m = only ] && E="$E ENOMOTO_MIP_INJECT_ONLY=1"
N=r3_step${k}_$m$(echo "$@" | tr -dc 'A-Z0-9' | tail -c 12)
NODE_LIMIT=1 $D/run_eno.sh roll3000 $N 60 1 $E "$@" >/dev/null
h1=$(grep "^HRND $((k+1)) " $D/logs/roll3000_h_nopre.err|head -1|awk '{print $4}')
echo "k=$k $m $*: start $(grep 'INJECT: added' $D/logs/$N.err | awk '{print $10}') ENO1 $(grep 'cut round 0' $D/logs/$N.err | head -1| awk '{print $5, $(NF-1)}') HiGHS1 $h1 | $(grep 'INJECT: round' $D/logs/$N.err | head -1 | cut -d: -f3)"
