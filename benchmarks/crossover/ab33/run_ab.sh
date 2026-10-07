#!/bin/bash
# LU の稠密切替の案の比較 (二段解法と内点法 + クロスオーバーの両方、各問題で交互に計測)。
#   A: 自動切替の条件を広げる (行数 2000 以上・直前の LU が 1 行 16 要素以上・密度 0.1)
#   C: すべての分解で密度 0.1 で切替える (ENOMOTO_LU_DENSE_SWITCH=0.1)
cd "$(dirname "$0")/../../.."
A="ENOMOTO_T_LU_DENSE_SWITCH_AUTO_MIN_M=2000,ENOMOTO_T_LU_DENSE_SWITCH_AUTO_LU_PER_ROW=16,ENOMOTO_T_LU_DENSE_SWITCH_AUTO=0.1"
C="ENOMOTO_LU_DENSE_SWITCH=0.1"
V="base:slope_intercept= A:slope_intercept=$A C:slope_intercept=$C base:ipm_crossover= A:ipm_crossover=$A C:ipm_crossover=$C"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab33/nk.jsonl >> benchmarks/crossover/ab33/nk.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab33/done.txt
