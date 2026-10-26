#!/bin/bash
cd "$(dirname "$0")/../.."
TS="ENOMOTO_MIP_TREE_SEP=1"
AH="ENOMOTO_MIP_CUT_AGE_HIGHS=1"
for rep in 1 2; do
for p in beasleyC3 mc11 neos-911970 hypothyroid-k1; do
  python runs/nsa/run1.py $p def $rep
  python runs/nsa/run1.py $p ts $rep $TS
  python runs/nsa/run1.py $p ah10 $rep $AH ENOMOTO_T_MIP_CUT_AGE_LIMIT=10
  python runs/nsa/run1.py $p ah50 $rep $AH ENOMOTO_T_MIP_CUT_AGE_LIMIT=50
  python runs/nsa/run1.py $p ts_ah50 $rep $TS $AH ENOMOTO_T_MIP_CUT_AGE_LIMIT=50
  python runs/nsa/run1.py $p ts_a50 $rep $TS ENOMOTO_T_MIP_CUT_AGE_LIMIT=50
done
done
echo BATCH1 DONE
