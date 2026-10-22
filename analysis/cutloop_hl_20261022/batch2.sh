#!/bin/bash
# 4 スレッド・60 秒: 副作用の 8 問を既定 / fb で 2 回ずつ (交互に)
D=$(cd $(dirname $0) && pwd)
for i in 1 2; do
for P in misc07 mik-250-20-75-4 neos5 qnet1 10teams beasleyC3 mc11 h80x6320d; do
  $D/run_eno.sh $P b2_${P}_def_$i 60 4
  $D/run_eno.sh $P b2_${P}_fb_$i 60 4 ENOMOTO_MIP_CUTSEL_FEEDBACK=1
done; done
