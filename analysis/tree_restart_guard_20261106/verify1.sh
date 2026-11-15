#!/bin/bash
# 試作の確かめ (whl3 = GUARD k・c 入り): 短い問題 60 秒を種 1・7 で 6 形、その後に swath3・pg5_34 を 600 秒で def・forcekc
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
for s in 1 7; do
for p in neos-860300 drayage-25-23 eil33-2 neos5 pk1 mc11; do
  python $R $p $s v_def
  python $R $p $s v_inc ENOMOTO_MIP_TREE_RESTART=inc
  python $R $p $s v_incc ENOMOTO_MIP_TREE_RESTART=inc ENOMOTO_MIP_TREE_RESTART_GUARD=c
  python $R $p $s v_force ENOMOTO_MIP_TREE_RESTART=force
  python $R $p $s v_forcekc ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kc
  python $R $p $s v_forcekcr ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kc ENOMOTO_MIP_RESTART_ROOT_CUTS=1
done; done
export RUN_TL=600
for s in 1 2; do
  python $R swath3 $s v_def
  python $R swath3 $s v_forcekc ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kc
done
python $R pg5_34 1 v_forcekc ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kc
python $R pg5_34 1 v_def
