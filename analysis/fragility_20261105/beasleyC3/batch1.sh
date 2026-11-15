#!/bin/bash
# 揺れの元を分ける計測 (a) 4 スレッド・種 0 を 4 回 (b) 1 スレッド・種 0〜4 (c) 4 スレッド・種 1〜10
cd "$(dirname "$0")"
D="ENOMOTO_MIP_FRAG_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
for r in 1 2 3 4; do python3 run1.py a$r 4 0 $D; done
for s in 0 1 2 3 4; do python3 run1.py b 1 $s $D; done
for s in 1 2 3 4 5 6 7 8 9 10; do python3 run1.py c 4 $s $D; done
echo BATCH1_DONE
