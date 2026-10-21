#!/bin/bash
# side3.sh: FJ_P3 (INC_LP_CAP を含めた後) の副作用。=1 は FJ だけ、=submip はサブ MIP も
cd "$(dirname "$0")"
for p in uccase12 neos-1456979 beasleyC3; do for i in 1 2; do
  ./run_cfg.sh $p 60 fjp3b ENOMOTO_MIP_FJ_P3=1
  ./run_cfg.sh $p 60 fjp3subb ENOMOTO_MIP_FJ_P3=submip
done; done
for i in 1 2; do
  ./run_cfg.sh physiciansched6-2 60 fjp3b ENOMOTO_MIP_FJ_P3=1 ENOMOTO_MIP_LOG=1
  ./run_cfg.sh physiciansched6-2 60 fjp3subb ENOMOTO_MIP_FJ_P3=submip ENOMOTO_MIP_LOG=1
done
