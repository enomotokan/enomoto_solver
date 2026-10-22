#!/bin/bash
# 基準: HiGHS (計測用) 1 回、ENOMOTO 既定・e2all 各 2 回 (ENOMOTO_MIP_DEBUG_SEP つき)
cd "$(dirname "$0")"
E2ALL="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
./highs_bin.sh physiciansched6-2 60 highs
for i in 1 2; do
  ./run_cfg.sh physiciansched6-2 60 def ENOMOTO_MIP_DEBUG_SEP=1
  ./run_cfg.sh physiciansched6-2 60 e2all ENOMOTO_MIP_DEBUG_SEP=1 $E2ALL
done
