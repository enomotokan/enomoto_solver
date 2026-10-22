#!/bin/bash
# physiciansched6-2: e2all / + ROOT_HSEL (年齢の削除を直した後) / + ROOT_HSEL の年齢なし。それぞれの回数を足す
cd "$(dirname "$0")"
E2ALL="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
for i in 1 2; do
  ./run_cfg.sh physiciansched6-2 60 e2all ENOMOTO_MIP_DEBUG_SEP=1 $E2ALL
  ./run_cfg.sh physiciansched6-2 60 e2hs ENOMOTO_MIP_DEBUG_SEP=1 $E2ALL ENOMOTO_MIP_ROOT_HSEL=1
  ./run_cfg.sh physiciansched6-2 60 e2hs ENOMOTO_MIP_DEBUG_SEP=1 $E2ALL ENOMOTO_MIP_ROOT_HSEL=1
  ./run_cfg.sh physiciansched6-2 60 e2hs_age0 ENOMOTO_MIP_DEBUG_SEP=1 $E2ALL ENOMOTO_MIP_ROOT_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
done
./run_cfg.sh physiciansched6-2 60 defhs ENOMOTO_MIP_DEBUG_SEP=1 ENOMOTO_MIP_ROOT_HSEL=1
