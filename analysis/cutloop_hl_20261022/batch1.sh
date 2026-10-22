#!/bin/bash
# 4 スレッド・60 秒: 担当の 4 問を既定 / fb で 2 回ずつ (交互に)
D=$(cd $(dirname $0) && pwd)
for i in 1 2; do
for P in roll3000 neos-911970 mzzv42z physiciansched6-2; do
  X=""; [ $P = physiciansched6-2 ] && X="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1"
  $D/run_eno.sh $P b1_${P}_def_$i 60 4 $X
  $D/run_eno.sh $P b1_${P}_fb_$i 60 4 $X ENOMOTO_MIP_CUTSEL_FEEDBACK=1
done; done
