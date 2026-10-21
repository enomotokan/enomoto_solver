#!/bin/bash
# 1. の切り分け: 7 問 × 9 設定 × 2 回 (回ごとに設定の順を回す)
D=$(cd $(dirname "$0") && pwd)
for k in 1 2; do
 for p in mc11 neos-960392 neos-911970 mzzv42z roll3000 uccase12 noswot; do
  for c in def gfr noG noR noF noX G F GR; do
   [ -f $D/logs/${p}_${c}_${k}.log ] && grep -q RESULT $D/logs/${p}_${c}_${k}.log && continue
   $D/erun.sh $p $c $k >> $D/logs/q1.txt
  done
 done
done
