#!/bin/bash
# side.sh: 副作用の計測 (60 秒・4 スレッド)。A = p5s + 同値の代入、B = p5s + カットの予算 (C1 のカットの部分)
cd "$(dirname "$0")"
P5S="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1"
PCR="ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_PROBE_REPRESOLVE=1"
CUTT="ENOMOTO_MIP_FACE_SEP=off ENOMOTO_T_MIP_CUT_TIME_FRAC=0.3 ENOMOTO_T_MIP_CUT_LP_TIME_FRAC=0.05 ENOMOTO_MIP_CUT_LP_CONTINUE=1"
for p in 30n20b8 beavma neos-860300 blp-ar98 qnet1 timtab1; do
  ./run_cfg.sh $p 60 p5s $P5S
  ./run_cfg.sh $p 60 pcr $P5S $PCR
done
for p in rout neos5 misc07 mik-250-20-75-4 nw04 qnet1 neos-911970; do
  ./run_cfg.sh $p 60 p5s $P5S
  ./run_cfg.sh $p 60 cutt $P5S $CUTT
done
