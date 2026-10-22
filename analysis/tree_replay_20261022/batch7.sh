#!/bin/bash
# HiGHS だけが解く 16 問 (benchmarks/highsonly16_*)、4 スレッド・60 秒、既定と mink を 1 回ずつ交互に
W=analysis/tree_replay_20261022
MINK="ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap"
for P in neos-911970 nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde mzzv42z neos-960392 roll3000 app1-1; do
  $W/run_any.sh $P $W/logs/h16/${P}_base 60 4
  $W/run_any.sh $P $W/logs/h16/${P}_mink 60 4 $MINK
done
