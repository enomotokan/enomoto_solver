#!/bin/bash
# 9 回目: 副作用 (根の時間の長い 3 問、60 秒、1 回ずつ): 既定・ff・mixoc・mixoc+ff
cd "$(dirname "$0")"
M="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
F="ENOMOTO_MIP_GR_RESTART=ff"
for p in neos-911970 roll3000 physiciansched6-2; do
  ./run_cfg.sh $p 60 def
  ./run_cfg.sh $p 60 ff $F
  ./run_cfg.sh $p 60 mixoc $M
  ./run_cfg.sh $p 60 mixocff $M $F
done
