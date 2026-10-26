#!/bin/bash
# 1 回目の比較: HiGHS 2 回・ENOMOTO 既定 2 回・mixoc 1 回・打ち切り値つき (両方)
cd "$(dirname "$0")"
M="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
./run_highs.sh mzzv42z 60 def
./run_cfg.sh mzzv42z 60 def
./run_highs.sh mzzv42z 60 def
./run_cfg.sh mzzv42z 60 def
./run_cfg.sh mzzv42z 60 mixoc $M
./run_highs.sh mzzv42z 60 cut objective_bound=-20539.5
./run_cfg.sh mzzv42z 60 cut ENOMOTO_MIP_CUTOFF=-20539.5
./run_cfg.sh mzzv42z 60 gr ENOMOTO_MIP_GR_RESTART=1
