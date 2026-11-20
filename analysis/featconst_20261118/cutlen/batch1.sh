#!/bin/bash
# cutlen (1): 代表 8 問 × {def, r8 (CUT_ROUNDS=8), r50 (CUT_ROUNDS=50・CUT_TIME_FRAC=0.2)} × 種 1・2、4 スレッド・60 秒。ログは logs/
cd "$(dirname "$0")"
for s in 1 2; do
for p in h80x6320d neos-911970 exp-1-500-5-5 neos-3381206-awhea p200x1188c qnet1 neos17 fiber; do
  ./one.sh $p ours $s 60 def
  ./one.sh $p ours $s 60 r8 ENOMOTO_T_MIP_CUT_ROUNDS=8
  ./one.sh $p ours $s 60 r50 ENOMOTO_T_MIP_CUT_ROUNDS=50 ENOMOTO_T_MIP_CUT_TIME_FRAC=0.2
done; done
