#!/bin/bash
# P5 = P3 + INC_LP_CAP + RINS_FJ_BOOST と既定の比較
cd "$(dirname "$0")"
P5="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1 ENOMOTO_MIP_RINS_FJ_BOOST=1"
for k in 1 2; do ./run_cfg.sh physiciansched6-2 60 p5 $P5; done
for p in uccase12 qiu eil33-2; do for k in 1 2; do ./run_cfg.sh $p 60 p5 $P5; done; done
for p in neos-911970 mas74 mas76 bppc4-08 pk1 khb05250 supportcase26 rout binkar10_1 fiber timtab1 flugpl; do
  ./run_cfg.sh $p 60 base
  ./run_cfg.sh $p 60 p5 $P5
done
