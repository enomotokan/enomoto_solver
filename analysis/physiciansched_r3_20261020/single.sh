#!/bin/bash
# single.sh <問題>...: 既定・p5s・p5s の 5 つを 1 つずつ有効にした形を 60 秒ずつ (ログつき)
cd "$(dirname "$0")"
F="ENOMOTO_MIP_INEQ_SINGLETON ENOMOTO_MIP_DUAL_UB ENOMOTO_MIP_CS_OBJ_PIVOT ENOMOTO_MIP_FJ_LP_REPAIR ENOMOTO_MIP_INC_LP_CAP"
for p in "$@"; do
  ./run_cfg.sh $p 60 def ENOMOTO_MIP_LOG=1
  ./run_cfg.sh $p 60 p5s $(for f in $F; do echo $f=1; done) ENOMOTO_MIP_LOG=1
  for f in $F; do ./run_cfg.sh $p 60 only_${f#ENOMOTO_MIP_} $f=1 ENOMOTO_MIP_LOG=1; done
done
