#!/bin/bash
# 副作用: 根のヒューリスティクスが短い・カットが効く問題を、既定と試作で 2 回ずつ
cd "$(dirname "$0")"
W=ENOMOTO_MIP_BG_CUTS_WAIT=1
for i in 1 2; do
  for p in physiciansched6-2 h80x6320d neos-911970 noswot roll3000; do
    ./run.sh $p ${p}_def_$i
    ./run.sh $p ${p}_w_$i $W
  done
done
