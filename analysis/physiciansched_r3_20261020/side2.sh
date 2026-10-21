#!/bin/bash
# side2.sh: ENOMOTO_MIP_FJ_P3=submip の副作用 (既定と比べる、60 秒)
cd "$(dirname "$0")"
for p in beasleyC3 uccase12 neos-1456979; do for i in 1 2; do
  ./run_cfg.sh $p 60 def2
  ./run_cfg.sh $p 60 fjp3sub ENOMOTO_MIP_FJ_P3=submip
done; done
for p in timtab1 bppc4-08 supportcase26 neos-911970 qnet1; do
  ./run_cfg.sh $p 60 def2
  ./run_cfg.sh $p 60 fjp3sub ENOMOTO_MIP_FJ_P3=submip
done
