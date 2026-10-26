#!/bin/bash
# nursesched-sprint02 の切り分け (G+R か F か) と P3 (P2 + 一般整数が 1% 以上の問題だけ GR_AUTO)
D=$(cd $(dirname "$0") && pwd)
for k in 4 5 6; do for c in def P1 GR F P3; do $D/erun.sh nursesched-sprint02 $c $k; done; done
for k in 1 2 3; do $D/erun.sh mzzv42z P3 $k; $D/erun.sh 30n20b8 P3 $k; done
