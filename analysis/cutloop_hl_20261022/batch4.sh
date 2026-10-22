#!/bin/bash
# 4 スレッド・60 秒: fbm (裏のカットのループでは使わない) を残りの問題で 2 回、mc11 は既定・fb ももう 1 回
D=$(cd $(dirname $0) && pwd)
for i in 1 2; do
for P in neos-911970 mzzv42z physiciansched6-2 neos5 qnet1 10teams beasleyC3 mc11; do
  X=""; [ $P = physiciansched6-2 ] && X="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1"
  $D/run_eno.sh $P b4_${P}_fbm_$i 60 4 $X ENOMOTO_MIP_CUTSEL_FEEDBACK=main
done; done
$D/run_eno.sh mc11 b4_mc11_def_3 60 4
$D/run_eno.sh mc11 b4_mc11_fb_3 60 4 ENOMOTO_MIP_CUTSEL_FEEDBACK=1
