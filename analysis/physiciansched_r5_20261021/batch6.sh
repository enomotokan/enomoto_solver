#!/bin/bash
# 試作の組 P5 = PROBE_EQUIV2 + EQUIV2_CUTS + EQUIV2_FJ + EQUIV2_TAB (60 秒) と副作用
cd "$(dirname "$0")"
P5="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
P=physiciansched6-2
./run_cfg.sh $P 60 p5 $P5
./run_cfg.sh $P 60 p5boost $P5 ENOMOTO_MIP_RINS_FJ_BOOST=1
./run_cfg.sh $P 60 p5 $P5
./run_cfg.sh $P 60 p5boost $P5 ENOMOTO_MIP_RINS_FJ_BOOST=1
./run_cfg.sh $P 60 def
for q in nw04 air03 hypothyroid-k1 neos-960392 mzzv42z; do
  for k in 1 2; do
    ./run_cfg.sh $q 60 def
    ./run_cfg.sh $q 60 p5 $P5
  done
done
