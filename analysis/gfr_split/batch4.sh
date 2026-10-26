#!/bin/bash
# X の直しの後 (Qf): 7 問 × 2 回 (neos-911970・neos-960392 は 3 回)、副作用の問題 × 2 回、mik-250 の 3 回目 (既定)
D=$(cd $(dirname "$0") && pwd)
run() { [ -f $D/logs/${1}_${2}_${3}.log ] && grep -q RESULT $D/logs/${1}_${2}_${3}.log && return; $D/erun.sh $1 $2 $3 >> $D/logs/q4.txt; }
for k in 1 2; do for p in neos-911970 neos-960392 mc11 mzzv42z roll3000 uccase12 noswot; do run $p Qf $k; done; done
run neos-911970 Qf 3; run neos-960392 Qf 3
run mik-250-20-75-4 def 3
for k in 1 2; do for p in qnet1 mik-250-20-75-4 rout h80x6320d blend2 timtab1; do run $p Qf $k; done; done
