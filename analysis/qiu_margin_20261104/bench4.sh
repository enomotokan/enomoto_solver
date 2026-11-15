#!/bin/bash
# bench4: 試作 QIU_FIND_STOP_GAP (余裕は既定のまま)。4 スレッド・60 秒・6 回
cd /home/user/enomoto_solver; A=analysis/qiu_margin_20261104/run.sh
for r in 1 2 3 4 5 6; do
  $A b4_fgap_r$r 4 60 ENOMOTO_MIP_QIU_FIND_STOP_GAP=1
done
