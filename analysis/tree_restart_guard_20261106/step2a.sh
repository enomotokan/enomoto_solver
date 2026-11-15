#!/bin/bash
# 続き (whl4 → whl5 (C の既定をカットの条件なしにした)。def・incc は whl4・whl5 で動きが同じ。種 1 の def・incc は whl4 で走った)
# 元: (whl4 = ガード C 入り): mc11・neos-860300・eil33-2・drayage-25-23 を 60 秒・種 1〜3 で def・incc・incC・forcekC
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
for s in 1 2 3; do
for p in mc11 neos-860300 eil33-2 drayage-25-23; do
  [ $s != 1 ] && python $R $p $s w_def
  [ $s != 1 ] && python $R $p $s w_incc ENOMOTO_MIP_TREE_RESTART=inc ENOMOTO_MIP_TREE_RESTART_GUARD=c
  python $R $p $s w_incC ENOMOTO_MIP_TREE_RESTART=inc ENOMOTO_MIP_TREE_RESTART_GUARD=C
  python $R $p $s w_forcekC ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kC
done; done
