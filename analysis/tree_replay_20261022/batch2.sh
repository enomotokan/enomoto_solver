#!/bin/bash
# なぞり + HiGHS の衝突カットを木の前にカットプールへ (ORACLE) + カットプールの伝播、と対照 (カットプールの伝播だけ)
W=analysis/tree_replay_20261022
R=$W/replay_h1_cut_r.txt
C="ENOMOTO_MIP_CUTOFF=-2.38805 ENOMOTO_MIP_BRANCH_REPLAY=$R ENOMOTO_MIP_REPLAY_LOG=1"
for k in 1 2 3; do
  $W/run_eno.sh $W/logs/rpo_oracle_$k 900 1 $C ENOMOTO_MIP_ORACLE_CUTS=$W/highs_conflict_cuts.txt ENOMOTO_MIP_CUTPOOL_PROP=1
  [ $k -le 2 ] && $W/run_eno.sh $W/logs/rpo_cpprop_$k 900 1 $C ENOMOTO_MIP_CUTPOOL_PROP=1
done
