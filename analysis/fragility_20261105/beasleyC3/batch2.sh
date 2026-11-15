#!/bin/bash
# 仮説の確認: 役の起動の warm 条件を早める (既存の調整値) と遅い回が消えるか。4 スレッド・種 1〜10
cd "$(dirname "$0")"
D="ENOMOTO_MIP_FRAG_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
for s in 1 2 3 4 5 6 7 8 9 10; do python3 run1.py warm005 4 $s $D ENOMOTO_T_MIP_HEUR_TRIG_WARM_FRAC=0.005; done
echo BATCH2_DONE
