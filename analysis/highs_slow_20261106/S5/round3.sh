#!/bin/bash
# 3 回目: pg5_34 の切り分け (最適値を打ち切り値に与えた回) と、木の中の再スタートの試作
cd "$(dirname "$0")"
python3 run1.py ours pg5_34 cutoff_s0 600 ENOMOTO_MIP_CUTOFF=-14339.35 ENOMOTO_T_MIP_SEED=0
for s in 0 1; do
  python3 run1.py ours pg5_34 tr1_s$s 600 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_TREE_RESTART=1 ENOMOTO_MIP_TREE_RESTART_LOG=1
done
python3 run1.py ours pg5_34 trforce_s0 600 ENOMOTO_T_MIP_SEED=0 ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_LOG=1
