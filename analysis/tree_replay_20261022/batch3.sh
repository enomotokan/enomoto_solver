#!/bin/bash
# 担当の問題: 4 スレッド・60 秒 (打ち切り値なし) 既定と試作を 3 回ずつ、1 スレッド・打ち切り値つきの部品の切り分け (300 秒)
W=analysis/tree_replay_20261022
ALL="ENOMOTO_MIP_LP_INT_BOUNDS=1 ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap ENOMOTO_MIP_CUTPOOL_PROP=1 ENOMOTO_MIP_PROP_FIXPOINT=1"
for k in 1 2 3; do
  $W/run_any.sh neos-2978193-inde $W/logs/t4_base_$k 60 4
  $W/run_any.sh neos-2978193-inde $W/logs/t4_kp_$k 60 4 $ALL
done
for drop in LP_INT_BOUNDS PROOF_KEEP_INT PROOF_CMIR PROOF_CMIR_POOL CUTPOOL_PROP PROP_FIXPOINT; do
  E=$(echo $ALL | tr ' ' '\n' | grep -v "ENOMOTO_MIP_$drop=" | tr '\n' ' ')
  $W/run_any.sh neos-2978193-inde $W/logs/abl_no_$drop 300 1 ENOMOTO_MIP_CUTOFF=-2.38805 $E
done
