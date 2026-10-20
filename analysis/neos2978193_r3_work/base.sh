#!/bin/bash
cd "$(dirname "$0")"
P5S="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1"
for i in 1 2; do
  echo "def_$i $(./run.sh def60_$i 60 eno)"
  echo "p5s_$i $(./run.sh p5s60_$i 60 eno $P5S)"
  echo "hig_$i $(./run.sh hig60_$i 60 highs)"
done
