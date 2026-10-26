#!/bin/bash
# 4 スレッド・60 秒: 13 問 × (既定 / DCD=1 / NORM=1 DCD=1) × 2 回
D=$(cd $(dirname $0) && pwd)
for i in 1 2; do
  $D/batch60.sh def_$i
  $D/batch60.sh dcd_$i ENOMOTO_MIP_CUTSEL_DCD=1
  $D/batch60.sh ndcd_$i ENOMOTO_MIP_CUTSEL_NORM=1 ENOMOTO_MIP_CUTSEL_DCD=1
done
