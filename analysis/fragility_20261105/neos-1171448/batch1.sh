#!/bin/bash
# 揺れの元の切り分け: (a) 4 スレッド・種 0 固定、(b) 1 スレッド・種を変える、(c) 4 スレッド・種を変える
cd "$(dirname "$0")"
DG="ENOMOTO_MIP_FRAG_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
for i in 1 2 3 4; do ./run.sh a$i 4 $DG ENOMOTO_T_MIP_SEED=0; done
for s in 0 1 2 3; do ./run.sh b$s 1 $DG ENOMOTO_T_MIP_SEED=$s; done
for s in 1 2 3 4 5 6 7 8 9 10; do ./run.sh c$s 4 $DG ENOMOTO_T_MIP_SEED=$s; done
