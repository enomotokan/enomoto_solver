#!/bin/bash
# 600 秒の回: 既定 (def、診断の計数だけ) と試作 (all = DIV + POOL + SYM) を問題ごとに交互に 2 回、その後に部品だけの回 (div・sym) を 1 回
cd /home/user/enomoto_solver/analysis/submip_impr_20261106/B
D="ENOMOTO_MIP_SUBB_COUNT=1 ENOMOTO_MIP_MEMBQ_HDIAG=1"
ALL="ENOMOTO_MIP_SUBB_DIV=1 ENOMOTO_MIP_SUBB_POOL=1 ENOMOTO_MIP_SUBB_SYM=1"
for r in 1 2; do
  for p in fiball neos-933966 trento1 nexp-150-20-8-5 thor50dday; do
    if [ $r = 1 ]; then ./run1.sh def$r $p $D; ./run1.sh all$r $p $D $ALL
    else ./run1.sh all$r $p $D $ALL; ./run1.sh def$r $p $D; fi
  done
done
for p in fiball neos-933966 trento1 nexp-150-20-8-5 thor50dday; do
  ./run1.sh div1 $p $D ENOMOTO_MIP_SUBB_DIV=1 ENOMOTO_MIP_SUBB_POOL=1
  ./run1.sh sym1 $p $D ENOMOTO_MIP_SUBB_SYM=1
done
