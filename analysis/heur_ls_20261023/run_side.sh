#!/bin/bash
# 56 問の fpe で悪く見えた問題を、既定・fp (EARLY_RENS なし)・fpe で 1 回ずつ
cd "$(dirname "$0")"
FP="ENOMOTO_MIP_LP_PROBE_FINISH=1 ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES=3"
for p in 30n20b8 neos-1456979 cvs16r128-89 mc11 neos-960392; do
  ./run_cfg.sh $p 60 def
  ./run_cfg.sh $p 60 fp $FP
  ./run_cfg.sh $p 60 fpe $FP ENOMOTO_MIP_EARLY_RENS=1
done
