#!/bin/bash
# cand9bisect_run.sh <問題> <回数> <名前> [外す切り替え ...] -- [追加 ENV=VAL ...]
# CAND から指定の切り替え (名前の一部、ENOMOTO_MIP_ を除く) を外して run_cfg.sh を回す
CAND="ENOMOTO_MIP_WORKER_HALT=1 ENOMOTO_MIP_LATE_NO_RESOLVE=1 ENOMOTO_MIP_LP_PROBE_FINISH=1 ENOMOTO_MIP_LP_PROBE_ROW_BASIS=1 ENOMOTO_MIP_POSTSOLVE_REPAIR=1 ENOMOTO_MIP_GR_RESTART=1 ENOMOTO_MIP_FJ_NOSOL_EXTEND=2 ENOMOTO_MIP_CUT_STRONG_WIN5=nofc ENOMOTO_MIP_CUT_INTOBJ_STALL=1 ENOMOTO_MIP_HEUR_MU_ROLE_OBS=1 ENOMOTO_MIP_SYM_SKIP_FIXED=1 ENOMOTO_MIP_CUT_AGG=1 ENOMOTO_MIP_RESTART_BASIS=1"
p=$1; k=$2; name=$3; shift 3
drop=(); extra=()
while [ $# -gt 0 ]; do [ "$1" = "--" ] && { shift; extra=("$@"); break; }; drop+=("$1"); shift; done
envs=()
for e in $CAND; do key=${e%%=*}; key=${key#ENOMOTO_MIP_}; skip=0; for d in "${drop[@]}"; do [ "$key" = "$d" ] && skip=1; done; [ $skip = 0 ] && envs+=("$e"); done
for i in $(seq $k); do bash $(dirname $0)/heur_ls_20261023/run_cfg.sh $p 60 $name "${envs[@]}" "${extra[@]}"; done
