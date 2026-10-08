#!/bin/bash
# 内点法を停止基準の後も数値が破綻する手前まで高精度化する案 (ENOMOTO_T_IPM_PUSH、各問題で交互に計測):
#   default: 今の既定
#   push3: 相対残差の最悪値が停止基準の 1e-3 倍になるか、伸びなくなるまで続ける
#   push5: 同じく 1e-5 倍
#   push3pc: push3 + 内点法を Pock–Chambolle でそろえた問題で解く (ENOMOTO_T_XO_IPM_PC=1)
cd "$(dirname "$0")/../../.."
V="default= push3=ENOMOTO_T_IPM_PUSH=1e-3 push5=ENOMOTO_T_IPM_PUSH=1e-5 push3pc=ENOMOTO_T_IPM_PUSH=1e-3,ENOMOTO_T_XO_IPM_PC=1"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab43/nk.jsonl >> benchmarks/crossover/ab43/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab43/done.txt
