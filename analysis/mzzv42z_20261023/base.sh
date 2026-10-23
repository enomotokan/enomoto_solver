#!/bin/bash
# 既定と mixoc を交互に回す: base.sh <回数> <接尾辞> [追加の ENV=VAL ...]
cd "$(dirname "$0")/../heur_ls_20261023"
n=$1; sfx=$2; shift 2
for i in $(seq $n); do
  ./run_cfg.sh mzzv42z 60 def$sfx "$@"
  ./run_cfg.sh mzzv42z 60 mixoc$sfx ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1 "$@"
done
