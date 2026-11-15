#!/bin/bash
# 2 回目: neos-960392 と swath1 で既存の試作・調整値を 3 回ずつ (種 0,1,2)。DEBUG_SUBMIP_STOP は表示だけ
cd "$(dirname "$0")"
D=ENOMOTO_MIP_DEBUG_SUBMIP_STOP=1
for s in 0 1 2; do
  python3 run1.py ours neos-960392 nostall_s$s 600 ENOMOTO_T_MIP_SEED=$s $D ENOMOTO_T_MIP_SUBMIP_STALL=0 ENOMOTO_MIP_NO_SUBMIP_OUTCOME_STOP=1
  python3 run1.py ours neos-960392 dbg_s$s 600 ENOMOTO_T_MIP_SEED=$s $D
  python3 run1.py ours swath1 ho2_s$s 600 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1
  python3 run1.py ours swath1 ho2tr_s$s 600 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1 ENOMOTO_MIP_TREE_RESTART=1
done
