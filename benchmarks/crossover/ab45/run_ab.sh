#!/bin/bash
# 内点法の高精度化 (打ち切り条件を直した後) と、内点法の前の行・列のそろえ方 (各問題で交互に計測):
#   default: 今の既定
#   push: 高精度化 (ENOMOTO_T_IPM_PUSH=1e-2、伸びない反復 1 回でやめる。同じ点での解き直しは数えない)
#   push_pc: push + Pock–Chambolle (ENOMOTO_T_XO_IPM_PC=1)
#   push_geo: push + 幾何平均 4 回 (ENOMOTO_T_XO_IPM_PC=2)
#   push_eq: push + 平衡化 (ENOMOTO_T_XO_IPM_PC=4)
cd "$(dirname "$0")/../../.."
P="ENOMOTO_T_IPM_PUSH=1e-2,ENOMOTO_T_IPM_PUSH_STALL=1"
V="default= push=$P push_pc=$P,ENOMOTO_T_XO_IPM_PC=1 push_geo=$P,ENOMOTO_T_XO_IPM_PC=2 push_eq=$P,ENOMOTO_T_XO_IPM_PC=4"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab45/nk.jsonl >> benchmarks/crossover/ab45/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab45/done.txt
