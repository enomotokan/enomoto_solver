#!/bin/bash
# 根のサブ MIP の書き出し (4 スレッド・60 秒)。MPS は scratchpad/dump/<問題>_<設定>_<番号>.mps
cd "$(dirname "$0")"
S=/tmp/claude-0/-home-user-enomoto-solver/fd9e178d-c0ad-57cb-89af-579c5837bc70/scratchpad/dump
E2="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
./run_cfg.sh physiciansched6-2 60 e2dump env $E2 ENOMOTO_MIP_SUBMIP_VERBOSE=1 ENOMOTO_MIP_SUBMIP_DUMP=$S/phys_e2_
for p in neos-960392 mzzv42z roll3000 beasleyC3; do
  ./run_cfg.sh $p 60 dump env ENOMOTO_MIP_SUBMIP_VERBOSE=1 ENOMOTO_MIP_SUBMIP_DUMP=$S/${p}_
done
