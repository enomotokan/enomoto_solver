#!/bin/bash
# bench2: 解発見スレッドを止めない (FIND_THREAD_NO_STOP) と既定 (小さい余裕) の遅れが消えるか。4 スレッド・60 秒・6 回ずつ
cd /home/user/enomoto_solver; A=analysis/qiu_margin_20261104/run.sh
for r in 1 2 3 4 5 6; do
  $A b2_defns_r$r 4 60 ENOMOTO_MIP_FIND_THREAD_NO_STOP=1
  $A b2_m0ns_r$r 4 60 ENOMOTO_MIP_FIND_THREAD_NO_STOP=1 ENOMOTO_MIP_MEM3_TINY_MARGIN=0
done
