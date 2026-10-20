#!/bin/bash
# 全候補の強分岐 (打ち切り値つき) と、既定 / 潜りの打ち切りなし の繰り返し
cd "$(dirname "$0")"
C="ENOMOTO_MIP_CUTOFF=-2.38805"
PQ="ENOMOTO_T_MIP_PLUNGE_QUOT=1 ENOMOTO_T_MIP_SIBLING_QUOT=1"
echo "sbfull 1 $(./run.sh cut_sbfull_1 300 eno $C ENOMOTO_MIP_SB_FULL=1 | cut -c1-200)"
for i in 3 4 5; do
  echo "def $i $(./run.sh cut_def_$i 300 eno $C | cut -c1-200)"
  echo "plq1 $i $(./run.sh cut_plq1_$i 300 eno $C $PQ | cut -c1-200)"
done
