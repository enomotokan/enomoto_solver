#!/bin/bash
# 続き (whl5): Mittelmann の 10 問を 600 秒で def・forcekC (種 1)、58 問を 60 秒で def・forcekC (種 1・2)、600 秒の種 2
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
P="pg5_34 swath3 neos-3627168-kasai csched008 sct2 uccase12 drayage-25-23 mc11 neos-860300 eil33-2"
export RUN_TL=600
for p in $P; do
  python $R $p 1 x_def
  python $R $p 1 x_forcekC ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kC
done
unset RUN_TL
export BENCH58_OUT=bench58_w5.jsonl
python analysis/tree_restart_guard_20261106/bench58.py def,forcekC 1
python analysis/tree_restart_guard_20261106/bench58.py def,forcekC 2
export RUN_TL=600
for p in $P; do
  python $R $p 2 x_def
  python $R $p 2 x_forcekC ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kC
done
