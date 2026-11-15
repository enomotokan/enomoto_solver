#!/bin/bash
# 頂点によるばらつき: batchv.sh 時間の上限 設定名=MPSEPの値 ... (設定名 def は既定)。問題は PROBS (環境変数、空白区切り)
# 出発の頂点 8 通り: r1・r2・r3 (種 1〜3、既定の競争)、rs (二段解法の勝ちに固定)、ri (内点法 + クロスオーバーの勝ちに固定)、
# m1・m2 (最適面の上で軸の入れ替えでずらした頂点から、種 1・2)、md (同じく遠い頂点 (mode d))
cd "$(dirname "$0")"
tl=$1; shift
PROBS=${PROBS:-"mc11 neos859080 beasleyC3 n2seq36q mzzv42z neos-1171448"}
RS="ENOMOTO_MIP_MC11FIX1_RACE=s ENOMOTO_T_MIP_MC11FIX1_RACE_SIZE=1e12"
RI="ENOMOTO_MIP_MC11FIX1_RACE=i ENOMOTO_T_MIP_MC11FIX1_RACE_SIZE=1e12"
for p in $PROBS; do
 for st in r1 r2 r3 rs ri m1 m2 md; do
  for cv in "$@"; do
   c=${cv%%=*}; v=${cv#*=}
   [ "$c" = def ] && ev="" || ev="ENOMOTO_MIP_MPSEP=$v"
   case $st in
     r1) sx="1";; r2) sx="2";; r3) sx="3";;
     rs) sx="1 $RS";; ri) sx="1 $RI";;
     m1) sx="1 ENOMOTO_MIP_MPSEP_START=1";; m2) sx="1 ENOMOTO_MIP_MPSEP_START=2";;
     md) sx="1 ENOMOTO_MIP_MPSEP_START=1 ENOMOTO_MIP_MPSEP_START_MODE=d";;
   esac
   seed=${sx%% *}; extra=""; [ "$sx" != "$seed" ] && extra=${sx#* }
   [ -s runs/v_${p}_${c}_${st}.out ] && continue
   ./run1.sh $p v_${p}_${c}_${st} 4 $seed $tl $extra $ev
  done
 done
done
