#!/bin/bash
# 4 スレッド・60 秒: 揺れの確認 (既定 / fb / fbm = 裏のカットのループでは使わない) を交互に 3 回
D=$(cd $(dirname $0) && pwd)
for i in 3 4 5; do
for P in misc07 mik-250-20-75-4 roll3000 h80x6320d; do
  $D/run_eno.sh $P b3_${P}_def_$i 60 4
  $D/run_eno.sh $P b3_${P}_fb_$i 60 4 ENOMOTO_MIP_CUTSEL_FEEDBACK=1
  $D/run_eno.sh $P b3_${P}_fbm_$i 60 4 ENOMOTO_MIP_CUTSEL_FEEDBACK=main
done; done
