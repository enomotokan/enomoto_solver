#!/bin/bash
# 今の develop で既定・E2C・HiGHS を並べる (60 秒)
cd "$(dirname "$0")"
P=physiciansched6-2
python3 highs_run.py ../../.miplib_cache/$P.mps 60 > logs/${P}_highs_1.log 2>&1; tail -1 logs/${P}_highs_1.log
./run_cfg.sh $P 60 def
./run_cfg.sh $P 60 e2c ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1
./run_cfg.sh $P 60 def
./run_cfg.sh $P 60 e2c ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1
./run_cfg.sh $P 60 e2 ENOMOTO_MIP_PROBE_EQUIV2=1
