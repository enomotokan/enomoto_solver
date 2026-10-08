#!/bin/bash
# 高精度化の目標を 1e-3 に戻した案 (第 45 回の 1e-2 では pilot.ja の途中の点が悪い基底になった) と HiGHS の比較 (各問題で交互に計測):
#   default: 今の既定
#   push: 高精度化 (ENOMOTO_T_IPM_PUSH=1e-3、伸びない反復 1 回でやめる。同じ点での解き直しと停止基準に達した反復は数えない)
#   push_pc: push + 内点法の前に Pock–Chambolle (ENOMOTO_T_XO_IPM_PC=1)
#   highs_ipm: HiGHS (内点法 + クロスオーバー)
cd "$(dirname "$0")/../../.."
P="ENOMOTO_T_IPM_PUSH=1e-3,ENOMOTO_T_IPM_PUSH_STALL=1"
V="default= push=$P push_pc=$P,ENOMOTO_T_XO_IPM_PC=1 highs_ipm="
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab46/nk.jsonl >> benchmarks/crossover/ab46/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab46/done.txt
