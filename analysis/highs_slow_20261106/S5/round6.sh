#!/bin/bash
# 6 回目: TREE_RESTART=force を pg5_34 の残りの種と、ほかの 3 問で 3 回ずつ
cd "$(dirname "$0")"
F="ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_LOG=1"
for s in 1 2; do python3 run1.py ours pg5_34 trforce_s$s 600 ENOMOTO_T_MIP_SEED=$s $F; done
for p in swath1 neos-960392 roll3000; do
  for s in 0 1 2; do python3 run1.py ours $p trforce_s$s 600 ENOMOTO_T_MIP_SEED=$s $F; done
done
