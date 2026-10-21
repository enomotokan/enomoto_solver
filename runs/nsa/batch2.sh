#!/bin/bash
# develop 0542795 + TREE_SEP の費用の判定 (HEAVY・CUT_AGE)
cd "$(dirname "$0")/../.."
TS="ENOMOTO_MIP_TREE_SEP=1"
for rep in 1 2; do
for p in noswot pk1 neos-911970 beasleyC3; do
  python runs/nsa/run1.py $p d2def $rep
  python runs/nsa/run1.py $p d2ts $rep $TS
  python runs/nsa/run1.py $p d2tsH $rep $TS ENOMOTO_T_MIP_TREE_SEP_HEAVY=1.25
  python runs/nsa/run1.py $p d2tsA $rep $TS ENOMOTO_T_MIP_TREE_SEP_CUT_AGE=30
  python runs/nsa/run1.py $p d2tsHA $rep $TS ENOMOTO_T_MIP_TREE_SEP_HEAVY=1.25 ENOMOTO_T_MIP_TREE_SEP_CUT_AGE=30
done
done
echo BATCH2 DONE
