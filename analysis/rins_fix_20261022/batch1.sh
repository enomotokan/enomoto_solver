#!/bin/bash
cd "$(dirname "$0")"
E2="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
P=physiciansched6-2
./run_cfg.sh $P 60 e2all $E2
./run_cfg.sh $P 60 e2all_cuts $E2 ENOMOTO_MIP_SUBMIP_CUTS=1
./run_cfg.sh $P 60 e2all_boost $E2 ENOMOTO_MIP_RINS_FJ_BOOST=1
./run_cfg.sh $P 60 e2all_cuts_boost $E2 ENOMOTO_MIP_SUBMIP_CUTS=1 ENOMOTO_MIP_RINS_FJ_BOOST=1
./run_cfg.sh $P 60 e2all_prop $E2 ENOMOTO_MIP_RINS_PROP=1
./run_cfg.sh $P 60 e2all_dive $E2 ENOMOTO_MIP_RINS_DIVE=1
./run_cfg.sh $P 60 def_cuts_boost ENOMOTO_MIP_SUBMIP_CUTS=1 ENOMOTO_MIP_RINS_FJ_BOOST=1
