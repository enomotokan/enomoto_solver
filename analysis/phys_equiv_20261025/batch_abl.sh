#!/bin/bash
# 形 D の部品を 1 つずつ外す (drayage-25-23・nursesched-sprint02、2 回ずつ、MIP_LOG つき)
cd "$(dirname "$0")"
E2="ENOMOTO_MIP_PROBE_EQUIV2=1"; CCE="ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1"
PR="ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6 ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO=3"; ALT="ENOMOTO_MIP_CUTSEL_PROBE_ALT=NS"
for r in 1 2; do for p in drayage-25-23 nursesched-sprint02; do
  ./run_cfg.sh $p 60 ab_def ENOMOTO_X=1
  ./run_cfg.sh $p 60 ab_D $E2 $CCE $PR $ALT
  ./run_cfg.sh $p 60 ab_noE2 $CCE $PR $ALT
  ./run_cfg.sh $p 60 ab_noCCE $E2 $PR $ALT
  ./run_cfg.sh $p 60 ab_noPR $E2 $CCE
  ./run_cfg.sh $p 60 ab_noALT $E2 $CCE $PR
done; done
echo ABL DONE
