#!/bin/bash
# 4 スレッド・60 秒: 既定 / HL2 (ENOMOTO_MIP_SUBMIP_HL2)。交互に回す
cd "$(dirname "$0")"
E2="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
for r in 1 2 3; do
  ./run_cfg.sh physiciansched6-2 60 e2 env $E2
  ./run_cfg.sh physiciansched6-2 60 e2hl2 env $E2 ENOMOTO_MIP_SUBMIP_HL2=1
done
for r in 1 2; do
  ./run_cfg.sh physiciansched6-2 60 def
  ./run_cfg.sh physiciansched6-2 60 hl2 env ENOMOTO_MIP_SUBMIP_HL2=1
  for p in neos-960392 mzzv42z roll3000 beasleyC3 nw04 supportcase26; do
    ./run_cfg.sh $p 60 def
    ./run_cfg.sh $p 60 hl2 env ENOMOTO_MIP_SUBMIP_HL2=1
  done
done
