#!/bin/bash
# 1 回目: 既定 (種 0,1,2) と HiGHS を 3 回ずつ交互に。速い 3 問の後に pg5_34
cd "$(dirname "$0")"
for p in neos-960392 swath1 roll3000 pg5_34; do
  for s in 0 1 2; do
    python3 run1.py ours $p def_s$s 600 ENOMOTO_T_MIP_SEED=$s
    python3 run1.py highs $p h$s 600
  done
done
