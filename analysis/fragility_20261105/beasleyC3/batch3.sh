#!/bin/bash
# 試作の比べ: ROBUST_LNSFREQ + ROBUST_IMPORT (計測のときの名前は ROBUST_STALL) (4 スレッド・種 1〜10、1 スレッド・種 0〜4)
cd "$(dirname "$0")"
D="ENOMOTO_MIP_FRAG_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
R="ENOMOTO_MIP_ROBUST_LNSFREQ=1 ENOMOTO_MIP_ROBUST_IMPORT=1"
for s in 1 2 3 4 5 6 7 8 9 10; do python3 run1.py rsi 4 $s $D $R; done
for s in 0 1 2 3 4; do python3 run1.py rsi 1 $s $D $R; done
echo BATCH3_DONE
