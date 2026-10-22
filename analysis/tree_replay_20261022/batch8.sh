#!/bin/bash
# 遅くなった 5 問の再計測: 4 スレッド・60 秒、既定と mink を 3 回ずつ交互に
W=analysis/tree_replay_20261022
MINK="ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap"
for k in 1 2 3; do
  for P in neos-911970 eil33-2 pk1 mc11 nursesched-sprint02; do
    $W/run_any.sh $P $W/logs/rep/${P}_base$k 60 4
    $W/run_any.sh $P $W/logs/rep/${P}_mink$k 60 4 $MINK
  done
done
