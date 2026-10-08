#!/bin/bash
# 内点法の高精度化 (ENOMOTO_T_IPM_PUSH=1e-3) の打ち切り条件 (各問題で交互に計測):
#   default: 今の既定
#   push3: 残差が伸びない反復が 2 回続いたらやめる (第 43 回と同じ)
#   push3s1: 1 回でやめる (ENOMOTO_T_IPM_PUSH_STALL=1)
cd "$(dirname "$0")/../../.."
V="default= push3=ENOMOTO_T_IPM_PUSH=1e-3 push3s1=ENOMOTO_T_IPM_PUSH=1e-3,ENOMOTO_T_IPM_PUSH_STALL=1"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab44/nk.jsonl >> benchmarks/crossover/ab44/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab44/done.txt
