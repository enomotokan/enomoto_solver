#!/bin/bash
# neos-960392: PCR のどちらの部分が崩すか (60 秒)
cd "$(dirname "$0")"
P=neos-960392
for i in 1 2; do
./run_cfg.sh $P 60 def ENOMOTO_MIP_LOG=1
./run_cfg.sh $P 60 rep ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_PROBE_REPRESOLVE=1
./run_cfg.sh $P 60 pcl ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_PROBE_CLIQUE=1
./run_cfg.sh $P 60 e2c ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1
done
