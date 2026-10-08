#!/bin/bash
# 内点法の前の Pock–Chambolle (そろえた空間で γ = 1) を非零の多い問題だけにかける (各問題で交互に計測):
#   default: 今の既定
#   pc6: 全問題に (第 49 回と同じ)
#   pc6_100k / pc6_200k: 内点法に渡す行列の非零が 10 万 / 20 万以上の問題だけ (ENOMOTO_T_XO_IPM_PC_MIN_NNZ)
cd "$(dirname "$0")/../../.."
P="ENOMOTO_T_XO_IPM_PC=1,ENOMOTO_T_XO_GAMMA_PDHG=6"
V="default= pc6=$P pc6_100k=$P,ENOMOTO_T_XO_IPM_PC_MIN_NNZ=100000 pc6_200k=$P,ENOMOTO_T_XO_IPM_PC_MIN_NNZ=200000"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab50/nk.jsonl >> benchmarks/crossover/ab50/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab50/done.txt
