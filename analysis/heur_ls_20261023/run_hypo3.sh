#!/bin/bash
# hypothyroid-k1: LP_PROBE_FINISH + ROOT_OBJ_PROBE_PASSES=3 (fp)、それに EARLY_RENS を足した形 (fpe) を既定・mixoc の上で 3 回ずつ交互に
cd "$(dirname "$0")"
MX="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
FP="ENOMOTO_MIP_LP_PROBE_FINISH=1 ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES=3"
for i in 1 2 3; do
  ./run_cfg.sh hypothyroid-k1 60 fp $FP
  ./run_cfg.sh hypothyroid-k1 60 mixoc_fp $MX $FP
  ./run_cfg.sh hypothyroid-k1 60 fpe $FP ENOMOTO_MIP_EARLY_RENS=1
  ./run_cfg.sh hypothyroid-k1 60 mixoc_fpe $MX $FP ENOMOTO_MIP_EARLY_RENS=1
done
