#!/bin/bash
# 第 50 回の Mittelmann の 6 問 (1 回ずつ、300 秒まで): 今の既定と pc6_100k。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
V="default= pc6_100k=ENOMOTO_T_XO_IPM_PC=1,ENOMOTO_T_XO_GAMMA_PDHG=6,ENOMOTO_T_XO_IPM_PC_MIN_NNZ=100000"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab50/mitt.jsonl >> benchmarks/crossover/ab50/mitt.log 2>&1
echo MITT_DONE >> benchmarks/crossover/ab50/done.txt
