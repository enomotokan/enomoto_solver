#!/bin/bash
# 診断 1: 既定 (DFSBB=2) で潜りの判定の子の下界の位置 u の集計を出す (ENOMOTO_MIP_DFSPQ=d、割合は変えない)。種 1。
D=$(cd "$(dirname "$0")" && pwd)
for a in "supportcase40 120" "rocI-4-11 120" "mcsched 120" "timtab1 60" "noswot 60" "roll3000 60"; do
  set -- $a; bash $D/one.sh $1 ours 1 $2 d ENOMOTO_MIP_DFSPQ=d
done
