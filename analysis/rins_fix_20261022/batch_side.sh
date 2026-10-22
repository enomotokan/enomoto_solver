#!/bin/bash
# 副作用: 既定 と 既定 + ROOT_SUBMIP_HL を交互に 2 回ずつ (4 スレッド・60 秒)
cd "$(dirname "$0")"
for r in 1 2; do
for P in neos-960392 mzzv42z hypothyroid-k1 nursesched-sprint02 h80x6320d uccase12 timtab1 binkar10_1 cvs16r128-89 beasleyC3; do
  if [ $r = 1 ]; then
    ./run_cfg.sh $P 60 def; ./run_cfg.sh $P 60 hl ENOMOTO_MIP_ROOT_SUBMIP_HL=1
  else
    ./run_cfg.sh $P 60 hl ENOMOTO_MIP_ROOT_SUBMIP_HL=1; ./run_cfg.sh $P 60 def
  fi
done; done
E2="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
for r in 1 2 3; do
./run_cfg.sh physiciansched6-2 60 e2all_hlf $E2 ENOMOTO_MIP_ROOT_SUBMIP_HL=1
./run_cfg.sh physiciansched6-2 60 def_hlf ENOMOTO_MIP_ROOT_SUBMIP_HL=1
done
