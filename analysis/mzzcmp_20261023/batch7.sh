#!/bin/bash
# 7 回目: 打ち切り値 (最適値) を与えて、再スタートの扱いで証明に近づくか (既存の切り替え)
cd "$(dirname "$0")"
C="ENOMOTO_MIP_CUTOFF=-20539.5"
./run_cfg.sh mzzv42z 60 cutgrreadd $C ENOMOTO_MIP_GR_RESTART=readd
./run_cfg.sh mzzv42z 60 cutgrff $C ENOMOTO_MIP_GR_RESTART=1 ENOMOTO_MIP_RESTART_FRAC_FREE=1
./run_cfg.sh mzzv42z 60 cutgrrf01 $C ENOMOTO_MIP_GR_RESTART=1 ENOMOTO_T_MIP_RESTART_FAC=0.01
