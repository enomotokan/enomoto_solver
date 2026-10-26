#!/bin/bash
# hypothyroid-k1: EARLY_RENS + ROOT_OBJ_PROBE_PASSES=3 (pe、LP_PROBE_FINISH なし) を既定・mixoc の上で 3 回ずつ交互に
cd "$(dirname "$0")"
MX="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
PE="ENOMOTO_MIP_EARLY_RENS=1 ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES=3"
for i in 1 2 3; do
  ./run_cfg.sh hypothyroid-k1 60 pe $PE
  ./run_cfg.sh hypothyroid-k1 60 mixoc_pe $MX $PE
done
