#!/bin/bash
# 最小の組の確認: min = PROOF_CMIR + PROOF_CMIR_POOL=nocap、mink = min + PROOF_KEEP_INT、kpnf = 全部から PROP_FIXPOINT を外す
W=analysis/tree_replay_20261022
MIN="ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap"
MINK="$MIN ENOMOTO_MIP_PROOF_KEEP_INT=1"
KPNF="ENOMOTO_MIP_LP_INT_BOUNDS=1 ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap ENOMOTO_MIP_CUTPOOL_PROP=1"
for k in 1 2; do
  $W/run_any.sh neos-2978193-inde $W/logs/c1_min_$k 300 1 ENOMOTO_MIP_CUTOFF=-2.38805 $MIN
  $W/run_any.sh neos-2978193-inde $W/logs/c1_mink_$k 300 1 ENOMOTO_MIP_CUTOFF=-2.38805 $MINK
  $W/run_any.sh neos-2978193-inde $W/logs/c1_kpnf_$k 300 1 ENOMOTO_MIP_CUTOFF=-2.38805 $KPNF
done
for k in 1 2 3; do
  $W/run_any.sh neos-2978193-inde $W/logs/t4_min_$k 60 4 $MIN
  $W/run_any.sh neos-2978193-inde $W/logs/t4_mink_$k 60 4 $MINK
done
