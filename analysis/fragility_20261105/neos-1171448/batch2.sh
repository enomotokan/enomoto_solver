#!/bin/bash
# 試作 ENOMOTO_MIP_ROBUST_STALL の比べ (4 スレッド、種 1..10。既定は batch1.sh の c1..c10)
cd "$(dirname "$0")"
DG="ENOMOTO_MIP_FRAG_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
for v in $*; do
  for s in 1 2 3 4 5 6 7 8 9 10; do ./run.sh ${v}_$s 4 $DG ENOMOTO_MIP_ROBUST_STALL=$v ENOMOTO_T_MIP_SEED=$s; done
done
