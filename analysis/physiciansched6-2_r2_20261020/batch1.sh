#!/bin/bash
cd "$(dirname "$0")"
P3="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1"
CAP="ENOMOTO_MIP_INC_LP_CAP=1"
CUT="ENOMOTO_MIP_FACE_SEP=off ENOMOTO_T_MIP_CUT_TIME_FRAC=0.3 ENOMOTO_T_MIP_CUT_LP_TIME_FRAC=0.1"
for p in physiciansched6-2 uccase12 qiu eil33-2; do
  for k in 1 2; do
    ./run_cfg.sh $p 60 base
    ./run_cfg.sh $p 60 p3 $P3
    ./run_cfg.sh $p 60 p3cap $P3 $CAP
    ./run_cfg.sh $p 60 p3cap_cut $P3 $CAP $CUT
  done
done
