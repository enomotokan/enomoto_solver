#!/bin/bash
# 4 回目: 試作 ENOMOTO_MIP_REDCOST_MULTI (根のカットのループの途中の LP の被約費用でも大域的に固定する)
cd "$(dirname "$0")"
R="ENOMOTO_MIP_REDCOST_MULTI=1"
./run_cfg.sh mps/mzzv42z_hpre.mps 60 cutgrrm ENOMOTO_MIP_CUTOFF=-20539.5 ENOMOTO_MIP_GR_RESTART=1 $R
./run_cfg.sh mzzv42z 60 rm $R
./run_cfg.sh mzzv42z 60 cutgrrm ENOMOTO_MIP_CUTOFF=-20539.5 ENOMOTO_MIP_GR_RESTART=1 $R
./run_cfg.sh mzzv42z 60 grrm ENOMOTO_MIP_GR_RESTART=1 $R
./run_cfg.sh mzzv42z 60 rm $R
