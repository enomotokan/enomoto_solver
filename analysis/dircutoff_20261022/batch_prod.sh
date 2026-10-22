#!/bin/bash
# 4 スレッド・60 秒: 13 問 × (積の形 α=0 / 積の形 α=0 + 外す基準) × 2 回
D=$(cd $(dirname $0) && pwd)
P="ENOMOTO_MIP_CUTSEL_DCD_PROD=1 ENOMOTO_T_CUTSEL_DCD_DENS_EXP=0"
for i in 1 2; do
  $D/batch60.sh prod0_$i $P
  $D/batch60.sh prod0keep_$i $P ENOMOTO_MIP_CUT_KEEP_DCD=1
done
