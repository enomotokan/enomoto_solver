#!/bin/bash
# 試作の確認: 7 問 × {P, PnoBG, BG} × 2 回、mzzv42z の既定・gfr の 3 回目
D=$(cd $(dirname "$0") && pwd)
run() { [ -f $D/logs/${1}_${2}_${3}.log ] && grep -q RESULT $D/logs/${1}_${2}_${3}.log && return; $D/erun.sh $1 $2 $3 >> $D/logs/q2.txt; }
run mzzv42z def 3; run mzzv42z gfr 3
for k in 1 2; do
 for p in neos-960392 mc11 neos-911970 mzzv42z roll3000 uccase12 noswot; do
  for c in P BG PnoBG; do run $p $c $k; done
 done
done
