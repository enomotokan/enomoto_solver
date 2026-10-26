#!/bin/bash
# 試作 Q (GR_AUTO=2・FACE_SEP_FC=off・RESTART_REF) の確認と、Q が働く 40 問の一部での副作用
D=$(cd $(dirname "$0") && pwd)
run() { [ -f $D/logs/${1}_${2}_${3}.log ] && grep -q RESULT $D/logs/${1}_${2}_${3}.log && return; $D/erun.sh $1 $2 $3 >> $D/logs/q3.txt; }
for k in 1 2 3; do run mc11 Q $k; done
for k in 1 2; do run neos-960392 Q $k; run neos-911970 Q $k; done
for k in 1 2; do for p in qnet1 mik-250-20-75-4 rout h80x6320d blend2 timtab1; do run $p def $k; run $p Q $k; done; done
