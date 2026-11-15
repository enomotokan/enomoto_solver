#!/bin/bash
# bench3: 試作 QIU_MARGIN_DEPTH と既定 (同じビルド)。4 スレッド・60 秒・6 回ずつ
cd /home/user/enomoto_solver; A=analysis/qiu_margin_20261104/run.sh
for r in 1 2 3 4 5 6; do
  $A b3_def_r$r 4 60
  $A b3_depth_r$r 4 60 ENOMOTO_MIP_QIU_MARGIN_DEPTH=1
done
