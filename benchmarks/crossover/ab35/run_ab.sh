#!/bin/bash
# LU の稠密切替の案 A2 (自動切替を行数 2000 以上・直前の LU が 1 行 16 要素以上に広げ、閾値は 0.3 のまま) の比較
# (二段解法と内点法 + クロスオーバー、各問題で交互に計測)。第 33 回の案 A (閾値 0.1) は maros-r7 で退行した。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab34/done.txt ]; do sleep 30; done
A2="ENOMOTO_T_LU_DENSE_SWITCH_AUTO_MIN_M=2000,ENOMOTO_T_LU_DENSE_SWITCH_AUTO_LU_PER_ROW=16"
V="base:slope_intercept= A2:slope_intercept=$A2 base:ipm_crossover= A2:ipm_crossover=$A2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab35/nk.jsonl >> benchmarks/crossover/ab35/nk.log 2>&1
# qap15 の進み具合 (600 秒)
S=/tmp/claude-0/-home-user-enomoto-solver/fde3ce0a-29a7-542c-bada-627585e735fc/scratchpad
for solver in ipm_crossover simplex; do
  ENOMOTO_T_LU_DENSE_SWITCH_AUTO_MIN_M=2000 ENOMOTO_T_LU_DENSE_SWITCH_AUTO_LU_PER_ROW=16 ENOMOTO_DEBUG_PROGRESS=1 ENOMOTO_DEBUG_CROSSOVER=1 \
    timeout 600 .venv/bin/python $S/check.py $S/qap/problem.mps $solver $S/x_tmp.npy > benchmarks/crossover/ab35/qap_${solver}_A2.log 2>&1
done
echo ALL_DONE >> benchmarks/crossover/ab35/done.txt
