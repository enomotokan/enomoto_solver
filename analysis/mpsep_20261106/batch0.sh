#!/bin/bash
# 探り: 設定の候補を 1 回ずつ (60 秒、種 1)
cd "$(dirname "$0")"
for p in mc11 beasleyC3 mzzv42z; do
  ./run1.sh $p x0_${p}_def 4 1 60
  ./run1.sh $p x0_${p}_oc 4 1 60 ENOMOTO_MIP_MPSEP=oc
  ./run1.sh $p x0_${p}_o 4 1 60 ENOMOTO_MIP_MPSEP=o
  ./run1.sh $p x0_${p}_c 4 1 60 ENOMOTO_MIP_MPSEP=c
  ./run1.sh $p x0_${p}_ocu 4 1 60 ENOMOTO_MIP_MPSEP=ocu
done
