#!/bin/bash
# drayage-25-23 の木に入る種 (3・7) を 60 秒で 4 形、その後に長い問題を 600 秒で tr1・force (swath3・pg5_34 は inc も)、種 1
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
for s in 3 7; do
  python $R drayage-25-23 $s def
  python $R drayage-25-23 $s tr1 ENOMOTO_MIP_TREE_RESTART=1
  python $R drayage-25-23 $s inc ENOMOTO_MIP_TREE_RESTART=inc
  python $R drayage-25-23 $s force ENOMOTO_MIP_TREE_RESTART=force
done
export RUN_TL=600
for p in swath3 pg5_34; do
  python $R $p 1 tr1 ENOMOTO_MIP_TREE_RESTART=1
  python $R $p 1 force ENOMOTO_MIP_TREE_RESTART=force
  python $R $p 1 inc ENOMOTO_MIP_TREE_RESTART=inc
done
for p in neos-3627168-kasai sct2 csched008 uccase12; do
  python $R $p 1 tr1 ENOMOTO_MIP_TREE_RESTART=1
  python $R $p 1 force ENOMOTO_MIP_TREE_RESTART=force
done
