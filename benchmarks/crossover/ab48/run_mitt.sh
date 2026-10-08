#!/bin/bash
# 第 48 回の Mittelmann の 6 問 (内点法 + クロスオーバーで解ける問題、1 回ずつ、300 秒まで): 今の既定と g0 (γ = 1)。
cd "$(dirname "$0")/../../.."
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
V="default= g0=ENOMOTO_T_XO_GAMMA_PDHG=0"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab48/mitt.jsonl >> benchmarks/crossover/ab48/mitt.log 2>&1
echo MITT_DONE >> benchmarks/crossover/ab48/done.txt
