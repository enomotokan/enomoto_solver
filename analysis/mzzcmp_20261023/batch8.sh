#!/bin/bash
# 8 回目: 手当て ENOMOTO_MIP_GR_RESTART=ff の評価 (mzzv42z、60 秒)。既定の def_1・def_2・mixoc_1 は batch1 のもの
cd "$(dirname "$0")"
M="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
F="ENOMOTO_MIP_GR_RESTART=ff"
./run_cfg.sh mzzv42z 60 ff $F
./run_cfg.sh mzzv42z 60 mixoc $M
./run_cfg.sh mzzv42z 60 mixocff $M $F
./run_cfg.sh mzzv42z 60 ff $F
./run_cfg.sh mzzv42z 60 mixocff $M $F
./run_cfg.sh mzzv42z 60 gr ENOMOTO_MIP_GR_RESTART=1
./run_cfg.sh mzzv42z 60 ffeq $F ENOMOTO_MIP_PROBE_EQUIV_FIX=1
./run_cfg.sh mzzv42z 60 ffeq $F ENOMOTO_MIP_PROBE_EQUIV_FIX=1
./run_cfg.sh mzzv42z 60 cutff $F ENOMOTO_MIP_CUTOFF=-20539.5
