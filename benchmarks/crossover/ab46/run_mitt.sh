#!/bin/bash
# 第 46 回の Mittelmann の 8 問 (1 回ずつ、300 秒まで)。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 datt256 scpm1 nug08-3rd fome13 ns1688926 ex10"
V="default= push=ENOMOTO_T_IPM_PUSH=1e-3,ENOMOTO_T_IPM_PUSH_STALL=1 highs_ipm="
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab46/mitt.jsonl >> benchmarks/crossover/ab46/mitt.log 2>&1
echo MITT_DONE >> benchmarks/crossover/ab46/done.txt
