#!/bin/bash
# hypothyroid-k1: HiGHS のログ 1 回と、既定・mixoc を 3 回ずつ交互に
cd "$(dirname "$0")"
python3 highs_log.py ../../.miplib_cache/hypothyroid-k1.mps 60 4 > logs/hypothyroid-k1_highs_1.log 2>&1
tail -1 logs/hypothyroid-k1_highs_1.log
for i in 1 2 3; do
  ./run_cfg.sh hypothyroid-k1 60 def
  ./run_cfg.sh hypothyroid-k1 60 mixoc ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1
done
