#!/bin/bash
# overrun_runs.sh <回の名前の接尾辞>: 3 問 × (既定・mixoc) × (WORKER_HALT なし・あり) を 1 回ずつ (ログなし、ENOMOTO_MIP_END_DIAG=1)
cd "$(dirname "$0")"
M="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
for p in physiciansched6-2 uccase12 mzzv42z; do
  NOLOG=1 ./tsrun.sh $p 60 def$1
  NOLOG=1 ./tsrun.sh $p 60 defh$1 ENOMOTO_MIP_WORKER_HALT=1
  NOLOG=1 ./tsrun.sh $p 60 mixoc$1 $M
  NOLOG=1 ./tsrun.sh $p 60 mixoch$1 $M ENOMOTO_MIP_WORKER_HALT=1
done
