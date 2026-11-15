#!/bin/bash
# 短い問題 (60 秒) を def・tr1 (=1)・inc・force で種 1・2
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
for s in 1 2; do
for p in neos-860300 drayage-25-23 eil33-2 neos5 pk1 mc11; do
  python $R $p $s def
  python $R $p $s tr1 ENOMOTO_MIP_TREE_RESTART=1
  python $R $p $s inc ENOMOTO_MIP_TREE_RESTART=inc
  python $R $p $s force ENOMOTO_MIP_TREE_RESTART=force
done; done
