#!/bin/bash
# 既存の調整値・試作と S4 の試作での切り分け (batch1 の後)
cd "$(dirname "$0")"
for s in 0 1 2; do python3 run1.py tg1_s$s neos-957323 ours $s ENOMOTO_MIP_S4_TINYGAP=1; done
python3 run1.py mr1_s0 neos-957323 ours 0 ENOMOTO_T_MIP_MAX_RESTARTS=1
python3 run1.py tg0_s0 neos-957323 ours 0 ENOMOTO_MIP_S4_TINYGAP=0
python3 run1.py tg2_s0 neos-957323 ours 0 ENOMOTO_MIP_S4_TINYGAP=2
for s in 0 1 2; do python3 run1.py norins_s$s cod105 ours $s ENOMOTO_MIP_NO_ROOT_RINS=1; done
python3 run1.py nosol50_s0 cod105 ours 0 ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP=50
python3 run1.py rel05_s0 cod105 ours 0 ENOMOTO_MIP_ROOT_SUBMIP_REL=0.5
python3 run1.py nolpp_s0 cod105 ours 0 ENOMOTO_MIP_NO_LP_PROBE=1
for s in 0 1 2; do python3 run1.py rfeqff_s$s mzzv42z ours $s ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1 ENOMOTO_MIP_PROBE_EQUIV_FIX=1 ENOMOTO_MIP_GR_RESTART=ff; done
