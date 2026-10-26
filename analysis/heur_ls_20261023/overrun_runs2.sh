#!/bin/bash
# overrun_runs2.sh <接尾辞>: 3 問 × (既定・mixoc) × (手当てなし・WORKER_HALT + LATE_NO_RESOLVE) を 1 回ずつ (ログなし、ENOMOTO_MIP_END_DIAG=1)
cd "$(dirname "$0")"
M="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
H="ENOMOTO_MIP_WORKER_HALT=1 ENOMOTO_MIP_LATE_NO_RESOLVE=1"
for p in ${PROBS:-physiciansched6-2 uccase12 mzzv42z}; do
  NOLOG=1 ./tsrun.sh $p 60 def$1
  NOLOG=1 ./tsrun.sh $p 60 defH$1 $H
  [ -n "$DEFONLY" ] && continue
  NOLOG=1 ./tsrun.sh $p 60 mixoc$1 $M
  NOLOG=1 ./tsrun.sh $p 60 mixocH$1 $M $H
done
