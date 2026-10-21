#!/bin/bash
cd "$(dirname "$0")"
P=physiciansched6-2
E="ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1"
./run_cfg.sh $P 60 e2cw200 $E ENOMOTO_T_MIP_CUT_WIDE_MAX=200
./run_cfg.sh $P 60 e2clp10 $E ENOMOTO_T_MIP_EQUIV2_CUT_LP_TIME_FRAC=0.1
./run_cfg.sh $P 60 e2cnow $E ENOMOTO_MIP_NO_CUT_WIDEN=1
./run_cfg.sh $P 60 e2c $E
