#!/bin/bash
# hypothyroid-k1: 手当て (LP_PROBE_FINISH と、EARLY_RENS を足した形) を既定・mixoc の上で 3 回ずつ交互に
cd "$(dirname "$0")"
MX="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
for i in 1 2 3; do
  ./run_cfg.sh hypothyroid-k1 60 fin ENOMOTO_MIP_LP_PROBE_FINISH=1
  ./run_cfg.sh hypothyroid-k1 60 mixoc_fin $MX ENOMOTO_MIP_LP_PROBE_FINISH=1
  ./run_cfg.sh hypothyroid-k1 60 finer ENOMOTO_MIP_LP_PROBE_FINISH=1 ENOMOTO_MIP_EARLY_RENS=1
  ./run_cfg.sh hypothyroid-k1 60 mixoc_finer $MX ENOMOTO_MIP_LP_PROBE_FINISH=1 ENOMOTO_MIP_EARLY_RENS=1
done
