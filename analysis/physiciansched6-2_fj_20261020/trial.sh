#!/bin/bash
# trial.sh <mps> [環境変数=値 ...]: 前処理後に FJ を種 20 個で回し、成功数と違反行数の分布を出す
mps=$1; shift
cd /tmp
out=$(env "$@" ENOMOTO_MIP_DEBUG_FJ_TRIALS=${TRIALS:-20} ENOMOTO_MIP_FJ_DEBUG=1 timeout 200 python -I /home/user/enomoto_solver/analysis/twovar_20261020/eno_run.py "$mps" ${TL:-2} 2>&1)
pre=$(echo "$out" | grep "^Presolve:" | head -1 | sed 's/.*-> //')
tr=$(echo "$out" | grep "FJ trials")
v=$(echo "$out" | grep "FJ2:" | head -${TRIALS:-20} | sed 's/.*least violated rows \([0-9]*\).*/\1/' | sort -n | tr '\n' ' ')
echo "pre=[$pre] $tr viol=[$v]"
