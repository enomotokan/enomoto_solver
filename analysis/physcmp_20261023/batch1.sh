#!/bin/bash
# physiciansched6-2 (60 秒、2 回ずつ) と副作用 3 問 (1 回ずつ)。ソルバーは 1 つずつ
cd "$(dirname "$0")"
MIX="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
AGG="ENOMOTO_MIP_CUT_AGG=1"
for i in 1 2; do
  ./run_cfg.sh physiciansched6-2 60 b_def
  ./run_cfg.sh physiciansched6-2 60 b_agg $AGG
  ./run_cfg.sh physiciansched6-2 60 b_mix $MIX
  ./run_cfg.sh physiciansched6-2 60 b_mixagg $MIX $AGG
done
for p in neos-911970 roll3000 mzzv42z; do
  ./run_cfg.sh $p 60 b_def
  ./run_cfg.sh $p 60 b_agg $AGG
done
