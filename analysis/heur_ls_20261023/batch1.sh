#!/bin/bash
# 比較: def (既定)・lm (Local-MIP、連鎖なし)・lmc (Local-MIP + 連鎖の手)。FJ と Local-MIP の最少違反行は FJ_DEBUG・LOCALMIP_DEBUG で出す
cd "$(dirname "$0")"
for rep in 1 2; do
for p in 30n20b8 neos-1456979 physiciansched6-2 roll3000 mzzv42z timtab1; do
  ./run_cfg.sh $p 60 def ENOMOTO_MIP_FJ_DEBUG=1
  ./run_cfg.sh $p 60 lm ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_LOCALMIP=1 ENOMOTO_MIP_LOCALMIP_DEBUG=1
  ./run_cfg.sh $p 60 lmc ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_LOCALMIP=1 ENOMOTO_MIP_LOCALMIP_CHAIN=1 ENOMOTO_MIP_LOCALMIP_DEBUG=1
done
done
echo BATCH1 DONE
