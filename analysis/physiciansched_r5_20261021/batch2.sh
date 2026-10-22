#!/bin/bash
# E2C の変種 (60 秒)。E2CF = E2C + ENOMOTO_MIP_EQUIV2_FJ
cd "$(dirname "$0")"
P=physiciansched6-2
E2CF="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1"
./run_cfg.sh $P 60 e2cf $E2CF
./run_cfg.sh $P 60 e2cf_tab $E2CF ENOMOTO_MIP_TAB_HIGHS_RULES=1
./run_cfg.sh $P 60 e2cf_noce $E2CF ENOMOTO_MIP_NO_CLIQUE_EXTEND=1
./run_cfg.sh $P 60 e2cf_hcm $E2CF ENOMOTO_MIP_HIGHS_CUTMGMT=1
./run_cfg.sh $P 60 e2cf $E2CF
